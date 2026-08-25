use image::{DynamicImage, GenericImageView, imageops::FilterType};
use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("failed to create media index directory {path}: {source}")]
    CreateDirectory {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to open media index {path}: {source}")]
    OpenIndex {
        path: PathBuf,
        #[source]
        source: rusqlite::Error,
    },

    #[error("media index database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("failed to set permissions on {path}: {source}")]
    SetPermissions {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to read image {path}: {source}")]
    ReadImage {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to decode image {label}: {message}")]
    DecodeImage { label: String, message: String },

    #[error("media index is corrupt: {message}")]
    CorruptIndex { message: String },

    #[error("media index does not exist at {path}; run `index build` first")]
    IndexNotFound { path: PathBuf },

    #[error("failed to remove media index {path}: {source}")]
    RemoveIndex {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum MediaKind {
    Image,
    Video,
}

impl MediaKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Image => "image",
            Self::Video => "video",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct IndexedFile {
    pub file_id: String,
    pub path: String,
    pub name: String,
    pub kind: MediaKind,
    pub mime_type: Option<String>,
    pub size: Option<u64>,
    pub etag: Option<String>,
    pub modified_at: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct FrameFingerprint {
    pub phash: u64,
    pub dhash: u64,
    pub color: [u8; 3],
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct FingerprintScore {
    pub phash_distance: u32,
    pub dhash_distance: u32,
    pub color_distance: u32,
    pub score: f64,
}

pub fn compare_fingerprints(
    query: &FrameFingerprint,
    candidate: &FrameFingerprint,
) -> FingerprintScore {
    let phash_distance = (query.phash ^ candidate.phash).count_ones();
    let dhash_distance = (query.dhash ^ candidate.dhash).count_ones();
    let color_distance = color_distance(query.color, candidate.color);
    FingerprintScore {
        phash_distance,
        dhash_distance,
        color_distance,
        score: similarity_score(phash_distance, dhash_distance, color_distance),
    }
}

#[derive(Debug, Clone)]
pub struct IndexedFrame {
    pub timestamp_ms: u64,
    pub fingerprint: FrameFingerprint,
}

#[derive(Debug, Clone, Serialize)]
pub struct VisualMatch {
    pub file_id: String,
    pub path: String,
    pub name: String,
    pub kind: MediaKind,
    pub mime_type: Option<String>,
    pub size: Option<u64>,
    pub timestamp_seconds: Option<f64>,
    pub phash_distance: u32,
    pub dhash_distance: u32,
    pub color_distance: u32,
    pub score: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct IndexStatus {
    pub path: PathBuf,
    pub schema_version: i64,
    pub file_count: u64,
    pub frame_count: u64,
    pub updated_at: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IndexedFileState {
    pub kind: MediaKind,
    pub unchanged: bool,
}

#[derive(Debug, Clone)]
pub struct FilePath {
    pub file_id: String,
    pub path: String,
}

pub fn index_directory(cache_dir: &Path, profile: &str) -> PathBuf {
    cache_dir
        .join("indexes")
        .join(safe_profile_component(profile))
}

pub fn index_path(cache_dir: &Path, profile: &str) -> PathBuf {
    index_directory(cache_dir, profile).join("index.sqlite")
}

pub fn media_cache_directory(cache_dir: &Path, profile: &str) -> PathBuf {
    index_directory(cache_dir, profile).join("media")
}

pub fn safe_profile_component(profile: &str) -> String {
    let value: String = profile
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.') {
                character
            } else {
                '_'
            }
        })
        .collect();
    if value.is_empty() || matches!(value.as_str(), "." | "..") {
        "profile".to_owned()
    } else {
        value
    }
}

pub fn media_kind(content_type: Option<&str>, path: &str) -> Option<MediaKind> {
    if let Some(content_type) = content_type {
        let normalized = content_type.to_ascii_lowercase();
        if normalized.starts_with("image/") {
            return Some(MediaKind::Image);
        }
        if normalized.starts_with("video/") {
            return Some(MediaKind::Video);
        }
    }

    let extension = Path::new(path)
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if matches!(
        extension.as_str(),
        "jpg" | "jpeg" | "png" | "webp" | "gif" | "bmp" | "tif" | "tiff"
    ) {
        return Some(MediaKind::Image);
    }
    if matches!(
        extension.as_str(),
        "mp4" | "mov" | "m4v" | "webm" | "mkv" | "avi" | "mts" | "m2ts"
    ) {
        return Some(MediaKind::Video);
    }
    None
}

pub fn fingerprint_image_bytes(bytes: &[u8], label: impl Into<String>) -> Result<FrameFingerprint> {
    let label = label.into();
    let image = image::load_from_memory(bytes).map_err(|source| Error::DecodeImage {
        label,
        message: source.to_string(),
    })?;
    Ok(fingerprint_image(&image))
}

pub fn fingerprint_image_file(path: &Path) -> Result<FrameFingerprint> {
    let bytes = fs::read(path).map_err(|source| Error::ReadImage {
        path: path.to_path_buf(),
        source,
    })?;
    fingerprint_image_bytes(&bytes, path.display().to_string())
}

pub fn fingerprint_image(image: &DynamicImage) -> FrameFingerprint {
    let (width, height) = image.dimensions();
    let rgb = image.resize_exact(32, 32, FilterType::Triangle).to_rgb8();
    let luma = image.resize_exact(32, 32, FilterType::Triangle).to_luma8();

    let phash = perceptual_hash(&luma);
    let dhash = difference_hash(image);
    let mut color = [0_u8; 3];
    let mut totals = [0_u64; 3];
    for pixel in rgb.pixels() {
        totals[0] += u64::from(pixel[0]);
        totals[1] += u64::from(pixel[1]);
        totals[2] += u64::from(pixel[2]);
    }
    let pixels = u64::from(rgb.width()) * u64::from(rgb.height());
    for (index, total) in totals.into_iter().enumerate() {
        color[index] = (total / pixels) as u8;
    }

    FrameFingerprint {
        phash,
        dhash,
        color,
        width,
        height,
    }
}

pub struct IndexDatabase {
    path: PathBuf,
    connection: Connection,
}

impl IndexDatabase {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|source| Error::CreateDirectory {
                path: parent.to_path_buf(),
                source,
            })?;
            set_owner_only_directory(parent).map_err(|source| Error::CreateDirectory {
                path: parent.to_path_buf(),
                source,
            })?;
        }

        let connection = Connection::open(path).map_err(|source| Error::OpenIndex {
            path: path.to_path_buf(),
            source,
        })?;
        connection.pragma_update(None, "foreign_keys", "ON")?;
        connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS index_meta (
                key TEXT PRIMARY KEY NOT NULL,
                value TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS files (
                file_id TEXT PRIMARY KEY NOT NULL,
                path TEXT NOT NULL,
                name TEXT NOT NULL,
                kind TEXT NOT NULL,
                mime_type TEXT,
                size INTEGER,
                etag TEXT,
                modified_at TEXT,
                indexed_at INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS frames (
                file_id TEXT NOT NULL,
                timestamp_ms INTEGER NOT NULL,
                phash TEXT NOT NULL,
                dhash TEXT NOT NULL,
                color_r INTEGER NOT NULL,
                color_g INTEGER NOT NULL,
                color_b INTEGER NOT NULL,
                width INTEGER NOT NULL,
                height INTEGER NOT NULL,
                PRIMARY KEY (file_id, timestamp_ms),
                FOREIGN KEY (file_id) REFERENCES files(file_id) ON DELETE CASCADE
            );
            CREATE INDEX IF NOT EXISTS frames_file_id_idx ON frames(file_id);
            CREATE INDEX IF NOT EXISTS files_path_idx ON files(path);
            INSERT INTO index_meta(key, value) VALUES ('schema_version', '1')
                ON CONFLICT(key) DO NOTHING;",
        )?;
        set_owner_only_file(path).map_err(|source| Error::SetPermissions {
            path: path.to_path_buf(),
            source,
        })?;
        Ok(Self {
            path: path.to_path_buf(),
            connection,
        })
    }

    pub fn open_existing(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Err(Error::IndexNotFound {
                path: path.to_path_buf(),
            });
        }
        Self::open(path)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn status(&self) -> Result<IndexStatus> {
        let schema_version = self
            .connection
            .query_row(
                "SELECT value FROM index_meta WHERE key = 'schema_version'",
                [],
                |row| row.get::<_, String>(0),
            )?
            .parse::<i64>()
            .map_err(|source| Error::CorruptIndex {
                message: format!("invalid schema version: {source}"),
            })?;
        let file_count = self
            .connection
            .query_row("SELECT COUNT(*) FROM files", [], |row| row.get::<_, i64>(0))?
            as u64;
        let frame_count = self
            .connection
            .query_row("SELECT COUNT(*) FROM frames", [], |row| {
                row.get::<_, i64>(0)
            })? as u64;
        let updated_at = self
            .connection
            .query_row(
                "SELECT value FROM index_meta WHERE key = 'updated_at'",
                [],
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .and_then(|value| value.parse::<u64>().ok());

        Ok(IndexStatus {
            path: self.path.clone(),
            schema_version,
            file_count,
            frame_count,
            updated_at,
        })
    }

    pub fn upsert_file(&mut self, file: &IndexedFile) -> Result<IndexedFileState> {
        let existing = self
            .connection
            .query_row(
                "SELECT kind, etag, size, modified_at,
                        EXISTS(SELECT 1 FROM frames WHERE frames.file_id = files.file_id)
                 FROM files WHERE file_id = ?1",
                params![file.file_id],
                |row| {
                    let kind: String = row.get(0)?;
                    let etag: Option<String> = row.get(1)?;
                    let size: Option<i64> = row.get(2)?;
                    let modified_at: Option<String> = row.get(3)?;
                    let has_frames: bool = row.get(4)?;
                    Ok((kind, etag, size, modified_at, has_frames))
                },
            )
            .optional()?;
        let size = file.size.and_then(|value| i64::try_from(value).ok());
        let unchanged =
            existing
                .as_ref()
                .is_some_and(|(kind, etag, old_size, modified_at, has_frames)| {
                    kind == file.kind.as_str()
                        && *has_frames
                        && match (etag.as_deref(), file.etag.as_deref()) {
                            (Some(old), Some(current)) => old == current,
                            _ => {
                                *old_size == size
                                    && modified_at.as_deref() == file.modified_at.as_deref()
                            }
                        }
                });
        let now = unix_timestamp();
        self.connection.execute(
            "INSERT INTO files(file_id, path, name, kind, mime_type, size, etag, modified_at, indexed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
             ON CONFLICT(file_id) DO UPDATE SET
                 path = excluded.path,
                 name = excluded.name,
                 kind = excluded.kind,
                 mime_type = excluded.mime_type,
                 size = excluded.size,
                 etag = excluded.etag,
                 modified_at = excluded.modified_at,
                 indexed_at = excluded.indexed_at",
            params![
                file.file_id,
                file.path,
                file.name,
                file.kind.as_str(),
                file.mime_type,
                size,
                file.etag,
                file.modified_at,
                now,
            ],
        )?;
        if !unchanged {
            self.connection.execute(
                "DELETE FROM frames WHERE file_id = ?1",
                params![file.file_id],
            )?;
        }
        Ok(IndexedFileState {
            kind: file.kind,
            unchanged,
        })
    }

    pub fn replace_frames(&mut self, file_id: &str, frames: &[IndexedFrame]) -> Result<()> {
        let transaction = self.connection.transaction()?;
        transaction.execute("DELETE FROM frames WHERE file_id = ?1", params![file_id])?;
        for frame in frames {
            transaction.execute(
                "INSERT INTO frames(file_id, timestamp_ms, phash, dhash, color_r, color_g, color_b, width, height)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    file_id,
                    i64::try_from(frame.timestamp_ms).unwrap_or(i64::MAX),
                    format!("{:016x}", frame.fingerprint.phash),
                    format!("{:016x}", frame.fingerprint.dhash),
                    i64::from(frame.fingerprint.color[0]),
                    i64::from(frame.fingerprint.color[1]),
                    i64::from(frame.fingerprint.color[2]),
                    i64::from(frame.fingerprint.width),
                    i64::from(frame.fingerprint.height),
                ],
            )?;
        }
        transaction.execute(
            "INSERT INTO index_meta(key, value) VALUES ('updated_at', ?1)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![unix_timestamp().to_string()],
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn touch_updated_at(&self) -> Result<()> {
        self.connection.execute(
            "INSERT INTO index_meta(key, value) VALUES ('updated_at', ?1)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![unix_timestamp().to_string()],
        )?;
        Ok(())
    }

    pub fn prune_missing_under(
        &mut self,
        root: &str,
        seen_file_ids: &HashSet<String>,
    ) -> Result<u64> {
        let root = root.trim_end_matches('/');
        let mut stale = Vec::new();
        let mut statement = self.connection.prepare("SELECT file_id, path FROM files")?;
        let rows = statement.query_map([], |row| {
            Ok(FilePath {
                file_id: row.get(0)?,
                path: row.get(1)?,
            })
        })?;
        for row in rows {
            let row = row?;
            let in_scope = root.is_empty()
                || root == "/"
                || row.path == root
                || row.path.starts_with(&format!("{root}/"));
            if in_scope && !seen_file_ids.contains(&row.file_id) {
                stale.push(row.file_id);
            }
        }
        drop(statement);
        for file_id in &stale {
            self.connection
                .execute("DELETE FROM files WHERE file_id = ?1", params![file_id])?;
        }
        Ok(stale.len() as u64)
    }

    pub fn search(
        &self,
        query: &FrameFingerprint,
        kind: Option<MediaKind>,
        limit: usize,
    ) -> Result<Vec<VisualMatch>> {
        let mut statement = self.connection.prepare(
            "SELECT f.file_id, f.path, f.name, f.kind, f.mime_type, f.size,
                    fr.timestamp_ms, fr.phash, fr.dhash, fr.color_r, fr.color_g, fr.color_b
             FROM frames fr
             JOIN files f ON f.file_id = fr.file_id
             WHERE (?1 IS NULL OR f.kind = ?1)",
        )?;
        let kind_filter = kind.map(MediaKind::as_str);
        let rows = statement.query_map(params![kind_filter], |row| {
            let kind: String = row.get(3)?;
            let kind = match kind.as_str() {
                "image" => MediaKind::Image,
                "video" => MediaKind::Video,
                other => {
                    return Err(rusqlite::Error::FromSqlConversionFailure(
                        3,
                        rusqlite::types::Type::Text,
                        Box::new(std::io::Error::other(format!("unknown media kind {other}"))),
                    ));
                }
            };
            let phash: String = row.get(7)?;
            let dhash: String = row.get(8)?;
            let phash = u64::from_str_radix(&phash, 16).map_err(|source| {
                rusqlite::Error::FromSqlConversionFailure(
                    7,
                    rusqlite::types::Type::Text,
                    Box::new(source),
                )
            })?;
            let dhash = u64::from_str_radix(&dhash, 16).map_err(|source| {
                rusqlite::Error::FromSqlConversionFailure(
                    8,
                    rusqlite::types::Type::Text,
                    Box::new(source),
                )
            })?;
            let timestamp_ms: i64 = row.get(6)?;
            let color = [
                row.get::<_, i64>(9)?.clamp(0, 255) as u8,
                row.get::<_, i64>(10)?.clamp(0, 255) as u8,
                row.get::<_, i64>(11)?.clamp(0, 255) as u8,
            ];
            let frame = FrameFingerprint {
                phash,
                dhash,
                color,
                width: 0,
                height: 0,
            };
            let phash_distance = (query.phash ^ frame.phash).count_ones();
            let dhash_distance = (query.dhash ^ frame.dhash).count_ones();
            let color_distance = color_distance(query.color, frame.color);
            let score = similarity_score(phash_distance, dhash_distance, color_distance);
            Ok(VisualMatch {
                file_id: row.get(0)?,
                path: row.get(1)?,
                name: row.get(2)?,
                kind,
                mime_type: row.get(4)?,
                size: row
                    .get::<_, Option<i64>>(5)?
                    .and_then(|value| u64::try_from(value).ok()),
                timestamp_seconds: if matches!(kind, MediaKind::Video) {
                    Some(timestamp_ms as f64 / 1000.0)
                } else {
                    None
                },
                phash_distance,
                dhash_distance,
                color_distance,
                score,
            })
        })?;
        let mut matches = Vec::new();
        for row in rows {
            matches.push(row?);
        }
        matches.sort_by(|left, right| {
            right
                .score
                .total_cmp(&left.score)
                .then_with(|| left.phash_distance.cmp(&right.phash_distance))
                .then_with(|| left.path.cmp(&right.path))
                .then_with(|| {
                    left.timestamp_seconds
                        .unwrap_or_default()
                        .total_cmp(&right.timestamp_seconds.unwrap_or_default())
                })
        });
        matches.truncate(limit);
        Ok(matches)
    }

    pub fn clear(&mut self) -> Result<()> {
        self.connection.execute_batch(
            "DELETE FROM frames;
             DELETE FROM files;
             DELETE FROM index_meta WHERE key = 'updated_at';",
        )?;
        Ok(())
    }
}

pub fn remove_index(path: &Path) -> Result<bool> {
    let Some(directory) = path.parent() else {
        return Ok(false);
    };
    if !directory.exists() {
        return Ok(false);
    }
    fs::remove_dir_all(directory).map_err(|source| Error::RemoveIndex {
        path: directory.to_path_buf(),
        source,
    })?;
    Ok(true)
}

fn perceptual_hash(image: &image::GrayImage) -> u64 {
    let mut coefficients = Vec::with_capacity(64);
    for u in 0..8 {
        for v in 0..8 {
            let mut sum = 0.0_f64;
            for x in 0..32 {
                for y in 0..32 {
                    let pixel = f64::from(image.get_pixel(x, y)[0]);
                    sum += pixel
                        * ((std::f64::consts::PI * (2.0 * x as f64 + 1.0) * u as f64) / 64.0).cos()
                        * ((std::f64::consts::PI * (2.0 * y as f64 + 1.0) * v as f64) / 64.0).cos();
                }
            }
            let alpha_u = if u == 0 { 1.0 / 2.0_f64.sqrt() } else { 1.0 };
            let alpha_v = if v == 0 { 1.0 / 2.0_f64.sqrt() } else { 1.0 };
            coefficients.push(0.25 * alpha_u * alpha_v * sum);
        }
    }
    let mut sorted = coefficients.clone();
    sorted.sort_by(f64::total_cmp);
    let median = sorted[sorted.len() / 2];
    coefficients.into_iter().fold(0_u64, |hash, coefficient| {
        (hash << 1) | u64::from(coefficient > median)
    })
}

fn difference_hash(image: &DynamicImage) -> u64 {
    let resized = image.resize_exact(9, 8, FilterType::Triangle).to_luma8();
    let mut hash = 0_u64;
    for y in 0..8 {
        for x in 0..8 {
            let left = resized.get_pixel(x, y)[0];
            let right = resized.get_pixel(x + 1, y)[0];
            hash = (hash << 1) | u64::from(left > right);
        }
    }
    hash
}

fn color_distance(left: [u8; 3], right: [u8; 3]) -> u32 {
    u32::from(left[0].abs_diff(right[0]))
        + u32::from(left[1].abs_diff(right[1]))
        + u32::from(left[2].abs_diff(right[2]))
}

fn similarity_score(phash_distance: u32, dhash_distance: u32, color_distance: u32) -> f64 {
    let distance = (f64::from(phash_distance) / 64.0) * 0.5
        + (f64::from(dhash_distance) / 64.0) * 0.3
        + (f64::from(color_distance) / 765.0) * 0.2;
    (1.0 - distance).clamp(0.0, 1.0)
}

fn unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

fn set_owner_only_directory(path: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}

fn set_owner_only_file(path: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgb};
    use tempfile::TempDir;

    #[test]
    fn image_extensions_are_classified() {
        assert_eq!(media_kind(None, "/a/photo.JPG"), Some(MediaKind::Image));
        assert_eq!(
            media_kind(Some("video/quicktime"), "/a/movie.bin"),
            Some(MediaKind::Video)
        );
        assert_eq!(media_kind(Some("text/plain"), "/a/readme.txt"), None);
    }

    #[test]
    fn identical_images_rank_first() -> Result<()> {
        let temp = TempDir::new().expect("temp dir");
        let path = temp.path().join("index.sqlite");
        let mut database = IndexDatabase::open(&path)?;
        let image =
            ImageBuffer::from_fn(64, 64, |x, y| Rgb([x as u8, y as u8, ((x + y) / 2) as u8]));
        let dynamic = DynamicImage::ImageRgb8(image);
        let fingerprint = fingerprint_image(&dynamic);
        database.upsert_file(&IndexedFile {
            file_id: "one".to_owned(),
            path: "/one.png".to_owned(),
            name: "one.png".to_owned(),
            kind: MediaKind::Image,
            mime_type: Some("image/png".to_owned()),
            size: Some(10),
            etag: Some("etag".to_owned()),
            modified_at: None,
        })?;
        database.replace_frames(
            "one",
            &[IndexedFrame {
                timestamp_ms: 0,
                fingerprint,
            }],
        )?;
        let matches = database.search(&fingerprint, Some(MediaKind::Image), 5)?;
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].phash_distance, 0);
        assert_eq!(matches[0].dhash_distance, 0);
        assert_eq!(matches[0].score, 1.0);
        Ok(())
    }

    #[test]
    fn changed_etag_invalidates_frames() -> Result<()> {
        let temp = TempDir::new().expect("temp dir");
        let path = temp.path().join("index.sqlite");
        let mut database = IndexDatabase::open(&path)?;
        let file = IndexedFile {
            file_id: "one".to_owned(),
            path: "/one.png".to_owned(),
            name: "one.png".to_owned(),
            kind: MediaKind::Image,
            mime_type: Some("image/png".to_owned()),
            size: Some(10),
            etag: Some("old".to_owned()),
            modified_at: None,
        };
        assert!(!database.upsert_file(&file)?.unchanged);
        database.replace_frames("one", &[])?;
        let changed = IndexedFile {
            etag: Some("new".to_owned()),
            ..file
        };
        assert!(!database.upsert_file(&changed)?.unchanged);
        assert_eq!(database.status()?.frame_count, 0);
        Ok(())
    }

    #[test]
    fn profile_names_are_safe_path_components() {
        assert_eq!(safe_profile_component("home/work"), "home_work");
        assert_eq!(safe_profile_component(""), "profile");
        assert_eq!(safe_profile_component("."), "profile");
        assert_eq!(safe_profile_component(".."), "profile");
    }

    #[test]
    fn files_without_frames_are_retried() -> Result<()> {
        let temp = TempDir::new().expect("temp dir");
        let path = temp.path().join("index.sqlite");
        let mut database = IndexDatabase::open(&path)?;
        let file = IndexedFile {
            file_id: "one".to_owned(),
            path: "/one.png".to_owned(),
            name: "one.png".to_owned(),
            kind: MediaKind::Image,
            mime_type: Some("image/png".to_owned()),
            size: Some(10),
            etag: Some("etag".to_owned()),
            modified_at: None,
        };

        assert!(!database.upsert_file(&file)?.unchanged);
        assert!(!database.upsert_file(&file)?.unchanged);
        Ok(())
    }
}
