use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use nextcloud::{ConfigPaths, Profile, WebDavClient};
use nextcloud_index::{
    FrameFingerprint, IndexDatabase, IndexedFile, IndexedFrame, MediaKind, VisualMatch,
    fingerprint_image_bytes, fingerprint_image_file, index_path, media_kind, remove_index,
};
use serde::Serialize;
use serde_json::{Value, json};
use tempfile::{NamedTempFile, TempDir};

use crate::commands::IndexMedia;
use crate::error::{CliError, CliResult};

#[derive(Debug, Clone)]
pub struct IndexOptions {
    pub root: String,
    pub media: IndexMedia,
    pub max_files: u32,
    pub video_sample_rate: u32,
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
    let mut database = IndexDatabase::open(&path)?;
    let entries = webdav.walk(&scope).await?;
    let selected_entries: Vec<_> = entries
        .into_iter()
        .filter(|entry| {
            media_kind(entry.content_type.as_deref(), &entry.path)
                .is_some_and(|kind| includes_kind(options.media, kind))
        })
        .collect();
    if selected_entries.len() > options.max_files as usize {
        return Err(CliError::IndexLimitExceeded {
            limit: options.max_files,
        });
    }

    let mut seen_file_ids = HashSet::new();
    let mut indexed_files = 0_u64;
    let mut skipped_files = 0_u64;
    for entry in &selected_entries {
        let kind = media_kind(entry.content_type.as_deref(), &entry.path).expect("filtered media");
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
        let state = database.upsert_file(&indexed_file)?;
        seen_file_ids.insert(file_id.clone());
        if state.unchanged {
            skipped_files += 1;
            continue;
        }

        let frames = match kind {
            MediaKind::Image => {
                let fingerprint = fingerprint_remote_image(webdav, entry).await?;
                vec![IndexedFrame {
                    timestamp_ms: 0,
                    fingerprint,
                }]
            }
            MediaKind::Video => {
                let temporary = download_to_temp(webdav, &entry.path).await?;
                sample_video_frames(temporary.path(), options.video_sample_rate)?
            }
        };
        database.replace_frames(&file_id, &frames)?;
        indexed_files += 1;
    }

    let pruned_files = if options.media == IndexMedia::All {
        database.prune_missing_under(&scope, &seen_file_ids)?
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
        scanned_files: selected_entries.len() as u64,
        indexed_files,
        skipped_files,
        pruned_files,
        file_count: status.file_count,
        frame_count: status.frame_count,
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

async fn fingerprint_remote_image(
    webdav: &WebDavClient,
    entry: &nextcloud::WebDavEntry,
) -> CliResult<FrameFingerprint> {
    if let Some(file_id) = entry.file_id.as_deref()
        && let Ok(preview) = webdav.preview(file_id, 320, 320).await
        && let Ok(fingerprint) = fingerprint_image_bytes(&preview, &entry.path)
    {
        return Ok(fingerprint);
    }
    let bytes = webdav.download(&entry.path).await?;
    Ok(fingerprint_image_bytes(&bytes, &entry.path)?)
}

async fn download_to_temp(webdav: &WebDavClient, path: &str) -> CliResult<NamedTempFile> {
    let mut temporary = NamedTempFile::new().map_err(|source| CliError::VideoExtractionFailed {
        message: format!("failed to create temporary video file: {source}"),
    })?;
    webdav
        .download_to_writer(path, temporary.as_file_mut())
        .await?;
    temporary
        .as_file_mut()
        .sync_all()
        .map_err(|source| CliError::VideoExtractionFailed {
            message: format!("failed to flush temporary video file: {source}"),
        })?;
    Ok(temporary)
}

fn sample_video_frames(path: &Path, frames_per_second: u32) -> CliResult<Vec<IndexedFrame>> {
    extract_video_frames(path, 0.0, None, frames_per_second)
}

fn refine_video_frames(
    path: &Path,
    center_seconds: f64,
    frames_per_second: u32,
) -> CliResult<Vec<IndexedFrame>> {
    let start = (center_seconds - 1.5).max(0.0);
    extract_video_frames(path, start, Some(3.0), frames_per_second)
}

fn extract_video_frames(
    input: &Path,
    start_seconds: f64,
    duration_seconds: Option<f64>,
    frames_per_second: u32,
) -> CliResult<Vec<IndexedFrame>> {
    if Command::new("ffmpeg").arg("-version").output().is_err() {
        return Err(CliError::VideoToolUnavailable);
    }
    let temporary = TempDir::new().map_err(|source| CliError::VideoExtractionFailed {
        message: format!("failed to create frame cache: {source}"),
    })?;
    let pattern = temporary.path().join("frame-%08d.jpg");
    let filter =
        format!("fps={frames_per_second},scale=320:320:force_original_aspect_ratio=decrease");
    let mut command = Command::new("ffmpeg");
    command
        .args(["-hide_banner", "-loglevel", "error", "-nostdin"])
        .args(["-ss", &format!("{start_seconds:.3}")])
        .arg("-i")
        .arg(input);
    if let Some(duration_seconds) = duration_seconds {
        command.args(["-t", &format!("{duration_seconds:.3}")]);
    }
    let output = command
        .args(["-vf", &filter, "-q:v", "4", "-vsync", "0"])
        .arg(&pattern)
        .output()
        .map_err(|source| CliError::VideoExtractionFailed {
            message: format!("failed to launch ffmpeg: {source}"),
        })?;
    if !output.status.success() {
        let message = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        return Err(CliError::VideoExtractionFailed {
            message: if message.is_empty() {
                format!("ffmpeg exited with status {}", output.status)
            } else {
                message
            },
        });
    }

    let mut frame_paths = Vec::new();
    for entry in
        fs::read_dir(temporary.path()).map_err(|source| CliError::VideoExtractionFailed {
            message: format!("failed to read extracted frames: {source}"),
        })?
    {
        let entry = entry.map_err(|source| CliError::VideoExtractionFailed {
            message: format!("failed to inspect extracted frame: {source}"),
        })?;
        if entry
            .path()
            .extension()
            .and_then(|extension| extension.to_str())
            == Some("jpg")
        {
            frame_paths.push(entry.path());
        }
    }
    frame_paths.sort();

    let mut frames = Vec::with_capacity(frame_paths.len());
    for (index, frame_path) in frame_paths.iter().enumerate() {
        let fingerprint = fingerprint_image_file(frame_path)?;
        let timestamp_ms =
            ((start_seconds + index as f64 / frames_per_second as f64) * 1000.0).round() as u64;
        frames.push(IndexedFrame {
            timestamp_ms,
            fingerprint,
        });
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
    let temporary = download_to_temp(webdav, &candidate.path).await?;
    let center = candidate.timestamp_seconds.unwrap_or_default();
    let frames = refine_video_frames(temporary.path(), center, 10)?;
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
