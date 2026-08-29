use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use nextcloud::{ConfigPaths, Profile, WebDavClient};
use nextcloud_index::{
    FrameFingerprint, IndexDatabase, IndexedFile, IndexedFrame, MediaKind, VisualMatch,
    fingerprint_image_bytes, fingerprint_image_file, fingerprint_rgb8, index_path, media_kind,
    remove_index,
};
use serde::Serialize;
use serde_json::{Value, json};
use tempfile::NamedTempFile;

use crate::commands::IndexMedia;
use crate::error::{CliError, CliResult};

#[derive(Debug, Clone)]
pub struct IndexOptions {
    pub root: String,
    pub media: IndexMedia,
    pub max_files: u32,
    pub video_sample_rate: u32,
    pub scratch_dir: Option<PathBuf>,
    pub update_only: bool,
}

#[derive(Debug, Clone)]
pub struct SearchOptions {
    pub root: String,
    pub media: IndexMedia,
    pub limit: u32,
    pub video_candidates: u32,
}

#[derive(Debug, Serialize)]
struct IndexRunOutput {
    profile: String,
    scope: String,
    media: String,
    backend: &'static str,
    index_path: PathBuf,
    scanned_files: u64,
    indexed_files: u64,
    skipped_files: u64,
    pruned_files: u64,
    file_count: u64,
    frame_count: u64,
    streaming: bool,
    scratch_limit_bytes: u64,
}

pub async fn index_status(paths: &ConfigPaths, profile: &Profile) -> CliResult<Value> {
    let path = index_path(&paths.cache_dir, &profile.name);
    if !path.exists() {
        return Err(CliError::LocalIndexUnavailable {
            profile: profile.name.clone(),
        });
    }
    let database = IndexDatabase::open_existing(&path)?;
    let status = database.status()?;
    Ok(json!({
        "profile": profile.name,
        "backend": "local_media_index",
        "status": status,
    }))
}

pub fn clear_index(paths: &ConfigPaths, profile: &Profile, yes: bool) -> CliResult<Value> {
    if !yes {
        return Err(CliError::ConfirmationRequired);
    }
    let path = index_path(&paths.cache_dir, &profile.name);
    let removed = remove_index(&path)?;
    Ok(json!({
        "profile": profile.name,
        "backend": "local_media_index",
        "index_path": path,
        "cleared": removed,
        "confirmed": true,
    }))
}

pub async fn build_index(
    webdav: &WebDavClient,
    paths: &ConfigPaths,
    profile: &Profile,
    options: &IndexOptions,
) -> CliResult<Value> {
    if options.max_files == 0 {
        return Err(CliError::IndexLimitExceeded {
            limit: options.max_files,
        });
    }
    if options.video_sample_rate == 0 || options.video_sample_rate > 60 {
        return Err(CliError::VideoExtractionFailed {
            message: "video sample rate must be between 1 and 60 frames per second".to_owned(),
        });
    }

    let scope = nextcloud::webdav::normalize_remote_path(&options.root)?;
    let path = index_path(&paths.cache_dir, &profile.name);
    if options.update_only && !path.exists() {
        return Err(CliError::LocalIndexUnavailable {
            profile: profile.name.clone(),
        });
    }
    let scratch_dir = options
        .scratch_dir
        .clone()
        .unwrap_or_else(|| paths.cache_dir.join("index-scratch"));
    ensure_scratch_directory(&scratch_dir)?;
    let mut database = IndexDatabase::open(&path)?;
    let run_id = database.begin_run();
    let mut pending_directories = vec![scope.clone()];
    let mut visited_directories = HashSet::new();
    let mut scanned_files = 0_u64;
    let mut indexed_files = 0_u64;
    let mut skipped_files = 0_u64;

    while let Some(directory) = pending_directories.pop() {
        if !visited_directories.insert(directory.clone()) {
            continue;
        }
        for entry in webdav.list(&directory).await? {
            if entry.is_dir {
                pending_directories.push(entry.path);
                continue;
            }
            let Some(kind) = media_kind(entry.content_type.as_deref(), &entry.path)
                .filter(|kind| includes_kind(options.media, *kind))
            else {
                continue;
            };
            scanned_files += 1;
            if scanned_files > u64::from(options.max_files) {
                return Err(CliError::IndexLimitExceeded {
                    limit: options.max_files,
                });
            }

            let file_id = entry
                .file_id
                .clone()
                .unwrap_or_else(|| format!("path:{}", entry.path));
            let indexed_file = IndexedFile {
                file_id: file_id.clone(),
                path: entry.path.clone(),
                name: entry.name.clone(),
                kind,
                mime_type: entry.content_type.clone(),
                size: entry.size,
                etag: entry.etag.clone(),
                modified_at: entry.modified_at.clone(),
            };
            let state = database.upsert_file_seen(&indexed_file, run_id)?;
            if state.unchanged {
                skipped_files += 1;
                continue;
            }

            let frames = match kind {
                MediaKind::Image => {
                    let fingerprint =
                        fingerprint_remote_image(webdav, &entry, &scratch_dir).await?;
                    vec![IndexedFrame {
                        timestamp_ms: 0,
                        fingerprint,
                    }]
                }
                MediaKind::Video => {
                    sample_remote_video_frames(
                        webdav,
                        &entry.path,
                        0.0,
                        None,
                        options.video_sample_rate,
                    )
                    .await?
                }
            };
            database.replace_frames(&file_id, &frames)?;
            indexed_files += 1;
        }
    }

    let pruned_files = if options.media == IndexMedia::All {
        database.prune_missing_under(&scope, run_id)?
    } else {
        0
    };
    database.touch_updated_at()?;
    let status = database.status()?;
    Ok(serde_json::to_value(IndexRunOutput {
        profile: profile.name.clone(),
        scope,
        media: media_label(options.media).to_owned(),
        backend: "local_media_index",
        index_path: path,
        scanned_files,
        indexed_files,
        skipped_files,
        pruned_files,
        file_count: status.file_count,
        frame_count: status.frame_count,
        streaming: true,
        scratch_limit_bytes: MAX_IMAGE_FALLBACK_BYTES,
    })?)
}

pub async fn search_image(
    webdav: &WebDavClient,
    paths: &ConfigPaths,
    profile: &Profile,
    query_path: &Path,
    options: &SearchOptions,
) -> CliResult<Value> {
    if !query_path.is_file() {
        return Err(CliError::LocalQueryNotFile {
            path: query_path.to_path_buf(),
        });
    }
    if options.limit == 0 || options.limit > 100 {
        return Err(CliError::InvalidLimit {
            value: options.limit,
        });
    }
    let scope = nextcloud::webdav::normalize_remote_path(&options.root)?;
    let path = index_path(&paths.cache_dir, &profile.name);
    if !path.exists() {
        return Err(CliError::LocalIndexUnavailable {
            profile: profile.name.clone(),
        });
    }
    let query = fingerprint_image_file(query_path)?;
    let database = IndexDatabase::open_existing(&path)?;
    let kind = selected_kind(options.media);
    let coarse_matches = database.search(&query, kind, usize::MAX)?;
    let mut refined_video_files = HashSet::new();
    let mut by_file = HashMap::<String, VisualMatch>::new();
    let mut refined_video_count = 0_u64;

    for candidate in coarse_matches {
        if !matches_scope(&candidate.path, &scope) {
            continue;
        }
        let mut candidate = candidate;
        if candidate.kind == MediaKind::Video
            && refined_video_files.len() < options.video_candidates as usize
            && refined_video_files.insert(candidate.file_id.clone())
        {
            candidate = refine_video_match(webdav, &candidate, &query).await?;
            refined_video_count += 1;
        }
        by_file
            .entry(candidate.file_id.clone())
            .and_modify(|current| {
                if candidate.score > current.score {
                    *current = candidate.clone();
                }
            })
            .or_insert(candidate);
    }

    let mut results: Vec<_> = by_file.into_values().collect();
    results.sort_by(|left, right| {
        right
            .score
            .total_cmp(&left.score)
            .then_with(|| left.path.cmp(&right.path))
            .then_with(|| {
                left.timestamp_seconds
                    .unwrap_or_default()
                    .total_cmp(&right.timestamp_seconds.unwrap_or_default())
            })
    });
    results.truncate(options.limit as usize);
    Ok(json!({
        "profile": profile.name,
        "scope": scope,
        "query": query_path,
        "media": media_label(options.media),
        "backend": "local_media_index",
        "limit": options.limit,
        "video_candidates": options.video_candidates,
        "refined_video_count": refined_video_count,
        "results": results,
        "count": results.len(),
    }))
}

const VIDEO_FRAME_WIDTH: u32 = 320;
const VIDEO_FRAME_HEIGHT: u32 = 320;
const MAX_IMAGE_FALLBACK_BYTES: u64 = 64 * 1024 * 1024;

async fn fingerprint_remote_image(
    webdav: &WebDavClient,
    entry: &nextcloud::WebDavEntry,
    scratch_dir: &Path,
) -> CliResult<FrameFingerprint> {
    if let Some(file_id) = entry.file_id.as_deref()
        && let Ok(preview) = webdav
            .preview(file_id, VIDEO_FRAME_WIDTH, VIDEO_FRAME_HEIGHT)
            .await
        && preview.len() <= usize::try_from(MAX_IMAGE_FALLBACK_BYTES).unwrap_or(usize::MAX)
        && let Ok(fingerprint) = fingerprint_image_bytes(&preview, &entry.path)
    {
        return Ok(fingerprint);
    }
    if entry
        .size
        .is_some_and(|size| size > MAX_IMAGE_FALLBACK_BYTES)
    {
        return Err(CliError::IndexScratchLimitExceeded {
            limit: MAX_IMAGE_FALLBACK_BYTES,
        });
    }

    let mut temporary =
        NamedTempFile::new_in(scratch_dir).map_err(|source| CliError::IndexScratchUnavailable {
            path: scratch_dir.to_path_buf(),
            message: source.to_string(),
        })?;
    let mut writer = BoundedFileWriter::new(temporary.as_file_mut(), MAX_IMAGE_FALLBACK_BYTES);
    let downloaded = webdav.download_to_writer(&entry.path, &mut writer).await;
    if writer.exceeded {
        return Err(CliError::IndexScratchLimitExceeded {
            limit: MAX_IMAGE_FALLBACK_BYTES,
        });
    }
    downloaded?;
    temporary
        .as_file_mut()
        .sync_all()
        .map_err(|source| CliError::IndexScratchUnavailable {
            path: scratch_dir.to_path_buf(),
            message: source.to_string(),
        })?;
    Ok(fingerprint_image_file(temporary.path())?)
}

fn ensure_scratch_directory(path: &Path) -> CliResult<()> {
    fs::create_dir_all(path).map_err(|source| CliError::IndexScratchUnavailable {
        path: path.to_path_buf(),
        message: source.to_string(),
    })?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700)).map_err(|source| {
            CliError::IndexScratchUnavailable {
                path: path.to_path_buf(),
                message: source.to_string(),
            }
        })?;
    }
    Ok(())
}

struct BoundedFileWriter<'a> {
    file: &'a mut File,
    written: u64,
    limit: u64,
    exceeded: bool,
}

impl<'a> BoundedFileWriter<'a> {
    fn new(file: &'a mut File, limit: u64) -> Self {
        Self {
            file,
            written: 0,
            limit,
            exceeded: false,
        }
    }
}

impl Write for BoundedFileWriter<'_> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let length = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
        if self.written.saturating_add(length) > self.limit {
            self.exceeded = true;
            return Err(io::Error::other("bounded index scratch limit exceeded"));
        }
        let written = self.file.write(bytes)?;
        self.written = self.written.saturating_add(written as u64);
        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
    }
}

async fn sample_remote_video_frames(
    webdav: &WebDavClient,
    path: &str,
    start_seconds: f64,
    duration_seconds: Option<f64>,
    frames_per_second: u32,
) -> CliResult<Vec<IndexedFrame>> {
    if Command::new("ffmpeg").arg("-version").output().is_err() {
        return Err(CliError::VideoToolUnavailable);
    }

    let filter = format!(
        "fps={frames_per_second},scale={VIDEO_FRAME_WIDTH}:{VIDEO_FRAME_HEIGHT}:force_original_aspect_ratio=decrease,pad={VIDEO_FRAME_WIDTH}:{VIDEO_FRAME_HEIGHT}:(ow-iw)/2:(oh-ih)/2"
    );
    let mut command = Command::new("ffmpeg");
    command
        .args(["-hide_banner", "-loglevel", "error", "-nostdin"])
        .arg("-i")
        .arg("pipe:0");
    if start_seconds > 0.0 {
        command.args(["-ss", &format!("{start_seconds:.3}")]);
    }
    if let Some(duration_seconds) = duration_seconds {
        command.args(["-t", &format!("{duration_seconds:.3}")]);
    }
    let mut child = command
        .args([
            "-vf", &filter, "-pix_fmt", "rgb24", "-f", "rawvideo", "-vsync", "0", "pipe:1",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|source| CliError::VideoExtractionFailed {
            message: format!("failed to launch ffmpeg: {source}"),
        })?;
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| CliError::VideoExtractionFailed {
            message: "ffmpeg did not expose stdin".to_owned(),
        })?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| CliError::VideoExtractionFailed {
            message: "ffmpeg did not expose stdout".to_owned(),
        })?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| CliError::VideoExtractionFailed {
            message: "ffmpeg did not expose stderr".to_owned(),
        })?;

    let frame_reader =
        std::thread::spawn(move || read_raw_video_frames(stdout, start_seconds, frames_per_second));
    let stderr_reader = std::thread::spawn(move || {
        let mut stderr = stderr.take(64 * 1024);
        let mut bytes = Vec::new();
        let _ = stderr.read_to_end(&mut bytes);
        bytes
    });
    let webdav = webdav.clone();
    let path = path.to_owned();
    let download_task = tokio::spawn(async move {
        let result = webdav.download_to_writer(&path, &mut stdin).await;
        drop(stdin);
        result
    });

    let download_result =
        download_task
            .await
            .map_err(|source| CliError::VideoExtractionFailed {
                message: format!("remote video download task failed: {source}"),
            })?;
    if download_result.is_err() {
        let _ = child.kill();
    }
    let frames_result = frame_reader
        .join()
        .map_err(|_| CliError::VideoExtractionFailed {
            message: "video frame reader panicked".to_owned(),
        })?;
    let stderr = stderr_reader.join().unwrap_or_default();
    let status = child
        .wait()
        .map_err(|source| CliError::VideoExtractionFailed {
            message: format!("failed to wait for ffmpeg: {source}"),
        })?;

    download_result?;
    if !status.success() {
        let message = String::from_utf8_lossy(&stderr).trim().to_owned();
        return Err(CliError::VideoExtractionFailed {
            message: if message.is_empty() {
                format!("ffmpeg exited with status {status}")
            } else {
                message
            },
        });
    }
    frames_result
}

#[cfg(test)]
fn sample_video_frames(path: &Path, frames_per_second: u32) -> CliResult<Vec<IndexedFrame>> {
    extract_video_frames(path, 0.0, None, frames_per_second)
}

#[cfg(test)]
fn extract_video_frames(
    input: &Path,
    start_seconds: f64,
    duration_seconds: Option<f64>,
    frames_per_second: u32,
) -> CliResult<Vec<IndexedFrame>> {
    if Command::new("ffmpeg").arg("-version").output().is_err() {
        return Err(CliError::VideoToolUnavailable);
    }
    let filter = format!(
        "fps={frames_per_second},scale={VIDEO_FRAME_WIDTH}:{VIDEO_FRAME_HEIGHT}:force_original_aspect_ratio=decrease,pad={VIDEO_FRAME_WIDTH}:{VIDEO_FRAME_HEIGHT}:(ow-iw)/2:(oh-ih)/2"
    );
    let mut command = Command::new("ffmpeg");
    command
        .args(["-hide_banner", "-loglevel", "error", "-nostdin"])
        .args(["-ss", &format!("{start_seconds:.3}")])
        .arg("-i")
        .arg(input);
    if let Some(duration_seconds) = duration_seconds {
        command.args(["-t", &format!("{duration_seconds:.3}")]);
    }
    let mut child = command
        .args([
            "-vf", &filter, "-pix_fmt", "rgb24", "-f", "rawvideo", "-vsync", "0", "pipe:1",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|source| CliError::VideoExtractionFailed {
            message: format!("failed to launch ffmpeg: {source}"),
        })?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| CliError::VideoExtractionFailed {
            message: "ffmpeg did not expose stdout".to_owned(),
        })?;
    let frames = read_raw_video_frames(stdout, start_seconds, frames_per_second);
    if frames.is_err() {
        let _ = child.kill();
    }
    let status = child
        .wait()
        .map_err(|source| CliError::VideoExtractionFailed {
            message: format!("failed to wait for ffmpeg: {source}"),
        })?;
    if !status.success() {
        return Err(CliError::VideoExtractionFailed {
            message: format!("ffmpeg exited with status {status}"),
        });
    }
    frames
}

fn read_raw_video_frames<R: Read>(
    mut stdout: R,
    start_seconds: f64,
    frames_per_second: u32,
) -> CliResult<Vec<IndexedFrame>> {
    let frame_size = (VIDEO_FRAME_WIDTH * VIDEO_FRAME_HEIGHT * 3) as usize;
    let mut buffer = vec![0_u8; frame_size];
    let mut frames = Vec::new();
    loop {
        match stdout.read_exact(&mut buffer) {
            Ok(()) => {
                let fingerprint = fingerprint_rgb8(
                    &buffer,
                    VIDEO_FRAME_WIDTH,
                    VIDEO_FRAME_HEIGHT,
                    "ffmpeg raw frame",
                )?;
                let timestamp_ms =
                    ((start_seconds + frames.len() as f64 / frames_per_second as f64) * 1000.0)
                        .round() as u64;
                frames.push(IndexedFrame {
                    timestamp_ms,
                    fingerprint,
                });
            }
            Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => break,
            Err(source) => {
                return Err(CliError::VideoExtractionFailed {
                    message: format!("failed to read ffmpeg frames: {source}"),
                });
            }
        }
    }
    if frames.is_empty() {
        return Err(CliError::VideoExtractionFailed {
            message: "ffmpeg produced no decodable frames".to_owned(),
        });
    }
    Ok(frames)
}

async fn refine_video_match(
    webdav: &WebDavClient,
    candidate: &VisualMatch,
    query: &FrameFingerprint,
) -> CliResult<VisualMatch> {
    let center = candidate.timestamp_seconds.unwrap_or_default();
    let frames = sample_remote_video_frames(
        webdav,
        &candidate.path,
        (center - 1.5).max(0.0),
        Some(3.0),
        10,
    )
    .await?;
    let Some(best) = frames.into_iter().min_by(|left, right| {
        let left_score = nextcloud_index::compare_fingerprints(query, &left.fingerprint);
        let right_score = nextcloud_index::compare_fingerprints(query, &right.fingerprint);
        right_score
            .score
            .total_cmp(&left_score.score)
            .then_with(|| left.timestamp_ms.cmp(&right.timestamp_ms))
    }) else {
        return Ok(candidate.clone());
    };
    let score = nextcloud_index::compare_fingerprints(query, &best.fingerprint);
    Ok(VisualMatch {
        timestamp_seconds: Some(best.timestamp_ms as f64 / 1000.0),
        phash_distance: score.phash_distance,
        dhash_distance: score.dhash_distance,
        color_distance: score.color_distance,
        score: score.score,
        ..candidate.clone()
    })
}

fn selected_kind(media: IndexMedia) -> Option<MediaKind> {
    match media {
        IndexMedia::Images => Some(MediaKind::Image),
        IndexMedia::Videos => Some(MediaKind::Video),
        IndexMedia::All => None,
    }
}

fn includes_kind(media: IndexMedia, kind: MediaKind) -> bool {
    selected_kind(media).is_none_or(|selected| selected == kind)
}

fn media_label(media: IndexMedia) -> &'static str {
    match media {
        IndexMedia::Images => "images",
        IndexMedia::Videos => "videos",
        IndexMedia::All => "all",
    }
}

fn matches_scope(path: &str, scope: &str) -> bool {
    scope == "/" || path == scope || path.starts_with(&format!("{scope}/"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn scope_matching_does_not_match_sibling_prefixes() {
        assert!(matches_scope("/J305/source.mov", "/J305"));
        assert!(!matches_scope("/J305-old/source.mov", "/J305"));
        assert!(matches_scope("/anywhere/file.png", "/"));
    }

    #[test]
    fn media_selection_is_explicit() {
        assert_eq!(selected_kind(IndexMedia::Images), Some(MediaKind::Image));
        assert!(includes_kind(IndexMedia::All, MediaKind::Video));
        assert!(!includes_kind(IndexMedia::Images, MediaKind::Video));
    }

    #[test]
    fn bounded_scratch_writer_rejects_bytes_after_limit() {
        let temporary = TempDir::new().expect("temporary directory");
        let path = temporary.path().join("fallback.bin");
        let mut file = File::create(path).expect("scratch file");
        let mut writer = BoundedFileWriter::new(&mut file, 4);

        writer.write_all(b"1234").expect("within scratch limit");
        let error = writer
            .write_all(b"5")
            .expect_err("limit should reject the write");
        assert_eq!(error.kind(), io::ErrorKind::Other);
        assert!(writer.exceeded);
    }

    #[test]
    fn video_sampling_produces_timestamped_frames_when_ffmpeg_is_available() {
        if Command::new("ffmpeg").arg("-version").output().is_err() {
            return;
        }
        let temporary = TempDir::new().expect("temporary directory");
        let video = temporary.path().join("sample.mp4");
        let output = Command::new("ffmpeg")
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-nostdin",
                "-f",
                "lavfi",
                "-i",
                "testsrc=size=64x64:rate=2",
                "-t",
                "1.5",
                "-pix_fmt",
                "yuv420p",
            ])
            .arg(&video)
            .output()
            .expect("launch ffmpeg");
        assert!(
            output.status.success(),
            "ffmpeg failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let frames = sample_video_frames(&video, 2).expect("sample video frames");
        assert!(frames.len() >= 2);
        assert_eq!(frames[0].timestamp_ms, 0);
        assert!(frames.iter().any(|frame| frame.timestamp_ms > 0));
    }
}
