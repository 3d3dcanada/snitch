//! Stripping, the pixel proof that makes the claim honest, and safe output paths.

use std::fs;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::{exif, jpeg, png};

/// Strip losslessly. Returns bytes removed. Refuses a format it cannot do without re-encoding.
///
/// JPEG and PNG only, and the refusal says so rather than falling back to a re-encode. The web
/// version of SNITCH handles WebP, HEIC and AVIF as well; this one does not, and no tool
/// description or README may imply otherwise.
pub fn strip(src: &Path, dst: &Path) -> Result<u64, String> {
    let ext = match src.extension().and_then(|e| e.to_str()) {
        Some(e) => format!(".{}", e.to_ascii_lowercase()),
        None => String::new(),
    };
    // The orientation is read before the surgery so it can be put back: a rotation the viewer
    // depends on is display, not metadata, even though it lives in the EXIF block.
    let orientation = exif::read_metadata(src)
        .ok()
        .as_ref()
        .and_then(exif::orientation_value)
        .map(exif::orientation_payload);

    match ext.as_str() {
        ".jpg" | ".jpeg" => {
            let segment = match orientation.as_deref() {
                Some(payload) => Some(jpeg::orientation_segment(payload)?),
                None => None,
            };
            jpeg::strip(src, dst, segment.as_deref())
        }
        ".png" => png::strip(src, dst, orientation.as_deref()),
        other => Err(format!(
            "{other} is not supported for lossless stripping. Only JPEG and PNG."
        )),
    }
}

/// Prove the strip did not touch the image. This is the check that makes the claim honest.
///
/// Decodes both files and hashes the dimensions, the colour type and the raw pixel bytes. It is a
/// per-run check on these two files, not a statement about the algorithm, and the difference
/// matters: the web version says "pixels untouched" on the strength of an offline verification,
/// and this one can say it because it just looked.
///
/// `None` means the comparison could not be made, which is not the same as `Some(false)`.
pub fn pixels_identical(a: &Path, b: &Path) -> Option<bool> {
    Some(pixel_digest(a)? == pixel_digest(b)?)
}

fn pixel_digest(path: &Path) -> Option<[u8; 32]> {
    // Sniff the format from the bytes. `image::open` guesses from the extension, and the file this
    // is asked about is usually the atomic temporary, whose extension is `.tmp`. That made every
    // strip refuse itself with "pixel comparison unavailable", which is the correct refusal for
    // the wrong reason: the check was broken, not the surgery.
    let image = image::ImageReader::open(path)
        .ok()?
        .with_guessed_format()
        .ok()?
        .decode()
        .ok()?;
    let mut hasher = Sha256::new();
    hasher.update(image.width().to_le_bytes());
    hasher.update(image.height().to_le_bytes());
    hasher.update(image.color().channel_count().to_le_bytes());
    hasher.update(image.color().bytes_per_pixel().to_le_bytes());
    hasher.update(image.to_rgba8().as_raw());
    Some(hasher.finalize().into())
}

// ----------------------------------------------------------------------------------------------
// Safe output. Shared by every caller so nobody reimplements the guards more weakly.
// ----------------------------------------------------------------------------------------------

pub fn outpath(src: &Path, out: Option<&Path>, suffix: &str) -> PathBuf {
    if let Some(out) = out {
        return out.to_path_buf();
    }
    let stem = src.file_stem().and_then(|s| s.to_str()).unwrap_or("out");
    let ext = src.extension().and_then(|e| e.to_str());
    let name = match ext {
        Some(ext) => format!("{stem}{suffix}.{ext}"),
        None => format!("{stem}{suffix}"),
    };
    src.parent().unwrap_or(Path::new(".")).join(name)
}

pub fn same_file(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => absolute(a) == absolute(b),
    }
}

fn absolute(p: &Path) -> PathBuf {
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        std::env::current_dir().unwrap_or_default().join(p)
    }
}

/// A temporary file beside the target, so the eventual rename is on the same filesystem and is
/// therefore atomic. A temp file in /tmp would make it a copy across devices, which is not.
pub fn temporary_sibling(path: &Path, keep_extension: bool) -> Result<PathBuf, String> {
    let directory = absolute(path)
        .parent()
        .unwrap_or(Path::new("."))
        .to_path_buf();
    let basename = path.file_name().and_then(|s| s.to_str()).unwrap_or("out");
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("out");
    let extension = path.extension().and_then(|e| e.to_str());
    let prefix = if keep_extension { stem } else { basename };
    let suffix = match (keep_extension, extension) {
        (true, Some(ext)) => format!(".{ext}"),
        _ => ".tmp".to_string(),
    };
    // No tempfile crate: a counter plus process id plus O_EXCL is the same guarantee in ten lines.
    for attempt in 0..4096u32 {
        let candidate = directory.join(format!(
            ".{prefix}.snitch-{}{attempt}{suffix}",
            std::process::id()
        ));
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate)
        {
            Ok(_) => return Ok(candidate),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(format!("{}: {e}", candidate.display())),
        }
    }
    Err("could not create a temporary file".into())
}

/// Strip into a sibling temp file, prove the pixels survived, then move it into place.
///
/// Nothing partial ever appears at the target: either the pixel check passes and the file is
/// replaced in one operation, or the temporary is removed and the target is untouched.
pub fn strip_atomic(source: &Path, target: &Path) -> Result<(u64, bool), String> {
    let temporary = temporary_sibling(target, false)?;
    let result = (|| {
        let removed = strip(source, &temporary)?;
        match pixels_identical(source, &temporary) {
            Some(true) => {}
            Some(false) => return Err("refusing to write output: pixels changed".into()),
            None => return Err("refusing to write output: pixel comparison unavailable".into()),
        }
        copy_permissions(source, &temporary)?;
        fs::rename(&temporary, target).map_err(|e| format!("{}: {e}", target.display()))?;
        Ok((removed, true))
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

fn copy_permissions(from: &Path, to: &Path) -> Result<(), String> {
    let metadata = fs::metadata(from).map_err(|e| format!("{}: {e}", from.display()))?;
    fs::set_permissions(to, metadata.permissions()).map_err(|e| format!("{}: {e}", to.display()))
}
