use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ImageSaveError {
    #[error("image payload is empty")]
    EmptyPayload,
    #[error("failed to decode image bytes")]
    Decode(#[from] base64::DecodeError),
    #[error("failed to resolve application directory: {0}")]
    ResolveApplicationDirectory(std::io::Error),
    #[error("application directory has no parent directory")]
    InvalidApplicationDirectory,
    #[error("failed to write image to {path}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

#[derive(Debug, Default)]
pub struct SaveImageOptions<'a> {
    pub file_stem: Option<&'a str>,
    pub mime_type: Option<&'a str>,
    pub output_dir: Option<&'a Path>,
}

pub fn save_base64_image(
    encoded: &str,
    options: SaveImageOptions<'_>,
) -> Result<PathBuf, ImageSaveError> {
    if encoded.trim().is_empty() {
        return Err(ImageSaveError::EmptyPayload);
    }

    let output_dir = resolve_output_dir(options.output_dir)?;
    fs::create_dir_all(&output_dir).map_err(|source| ImageSaveError::Io {
        path: output_dir.clone(),
        source,
    })?;

    let extension = extension_from_mime(options.mime_type);
    let file_name = match options.file_stem {
        Some(stem) => format!("{stem}.{extension}"),
        None => format!("mawaku-image-{}.{}", timestamp_suffix(), extension),
    };

    let path = output_dir.join(file_name);
    let bytes = BASE64_STANDARD
        .decode(encoded)
        .map_err(ImageSaveError::Decode)?;

    fs::write(&path, &bytes).map_err(|source| ImageSaveError::Io {
        path: path.clone(),
        source,
    })?;

    Ok(path)
}

fn resolve_output_dir(dir: Option<&Path>) -> Result<PathBuf, ImageSaveError> {
    if let Some(path) = dir {
        return Ok(path.to_path_buf());
    }

    let current_exe =
        std::env::current_exe().map_err(ImageSaveError::ResolveApplicationDirectory)?;
    let parent = current_exe
        .parent()
        .ok_or(ImageSaveError::InvalidApplicationDirectory)?;
    Ok(parent.to_path_buf())
}

fn extension_from_mime(mime_type: Option<&str>) -> &'static str {
    match mime_type
        .unwrap_or("image/png")
        .to_ascii_lowercase()
        .as_str()
    {
        "image/jpeg" | "image/jpg" => "jpg",
        "image/webp" => "webp",
        "image/gif" => "gif",
        "image/png" => "png",
        _ => "bin",
    }
}

fn timestamp_suffix() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests;
