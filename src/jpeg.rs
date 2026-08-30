//! JPEG marker surgery: drop the application segments that carry metadata, keep the ones that
//! decide how the image looks.
//!
//! A JPEG is a marker stream. APP0 through APP15 and COM are where EXIF, XMP, IPTC and C2PA live,
//! and dropping them leaves the entropy-coded scan untouched, so the decoded pixels are identical.
//! Three application segments are not metadata and must survive: JFIF/JFXX identifies the stream,
//! ICC_PROFILE decides colour, and Adobe APP14 describes the colour transform many CMYK JPEGs
//! depend on. Dropping those three is how a "lossless" strip silently shifts an image's colour.

use std::fs;
use std::path::Path;

/// APP0..APP15 and COM. Everything in this set is metadata unless `keep_application_segment` says
/// otherwise.
fn is_metadata_marker(marker: u8) -> bool {
    (0xE0..=0xEF).contains(&marker) || marker == 0xFE
}

fn keep_application_segment(marker: u8, payload: &[u8]) -> bool {
    (marker == 0xE0 && (payload.starts_with(b"JFIF\x00") || payload.starts_with(b"JFXX\x00")))
        || (marker == 0xE2 && payload.starts_with(b"ICC_PROFILE\x00"))
        || (marker == 0xEE && payload.starts_with(b"Adobe"))
}

/// Wrap a raw EXIF payload as an APP1 segment.
pub fn orientation_segment(payload: &[u8]) -> Result<Vec<u8>, String> {
    if payload.len() + 2 > 0xFFFF {
        return Err("orientation EXIF segment is too large".into());
    }
    let mut out = vec![0xFF, 0xE1];
    out.extend_from_slice(&((payload.len() + 2) as u16).to_be_bytes());
    out.extend_from_slice(payload);
    Ok(out)
}

/// Drop private and application metadata while retaining display-critical segments.
///
/// Returns bytes removed. `orientation` is an optional APP1 segment, already framed, reinserted so
/// a rotation the viewer depends on survives.
pub fn strip(src: &Path, dst: &Path, orientation: Option<&[u8]>) -> Result<u64, String> {
    let data = fs::read(src).map_err(|e| format!("{}: {e}", src.display()))?;
    if data.len() < 2 || data[0] != 0xFF || data[1] != 0xD8 {
        return Err("not a JPEG".into());
    }

    let mut out: Vec<u8> = vec![0xFF, 0xD8];
    let mut i = 2usize;
    let mut inserted_orientation = false;
    let mut saw_scan = false;

    while i + 1 < data.len() {
        if data[i] != 0xFF {
            return Err("malformed JPEG marker stream".into());
        }
        let marker_start = i;
        while i < data.len() && data[i] == 0xFF {
            i += 1;
        }
        if i >= data.len() {
            return Err("truncated JPEG marker".into());
        }
        let marker = data[i];
        i += 1;
        if marker == 0x00 {
            return Err("unexpected stuffed byte before JPEG scan".into());
        }
        // Standalone markers carry no length: TEM, SOI, EOI and the restart markers.
        if matches!(marker, 0x01 | 0xD8 | 0xD9) || (0xD0..=0xD7).contains(&marker) {
            out.extend_from_slice(&data[marker_start..i]);
            continue;
        }
        if i + 2 > data.len() {
            return Err("truncated JPEG segment length".into());
        }
        let seg_len = u16::from_be_bytes([data[i], data[i + 1]]) as usize;
        if seg_len < 2 {
            return Err("invalid JPEG segment length".into());
        }
        let segment_end = i + seg_len;
        if segment_end > data.len() {
            return Err("truncated JPEG segment".into());
        }

        if marker == 0xDA {
            // Start of scan: everything from here is image data and is copied verbatim.
            if let Some(seg) = orientation {
                if !inserted_orientation {
                    out.extend_from_slice(seg);
                }
            }
            if !contains(&data[segment_end..], &[0xFF, 0xD9]) {
                return Err("JPEG scan has no end marker".into());
            }
            out.extend_from_slice(&data[marker_start..]);
            saw_scan = true;
            break;
        }

        let payload = &data[i + 2..segment_end];
        let keep = !is_metadata_marker(marker) || keep_application_segment(marker, payload);
        // JFIF must stay first, so the orientation segment goes in after APP0 rather than before.
        if let Some(seg) = orientation {
            if !inserted_orientation && marker != 0xE0 {
                out.extend_from_slice(seg);
                inserted_orientation = true;
            }
        }
        if keep {
            out.extend_from_slice(&data[marker_start..segment_end]);
        }
        i = segment_end;
    }
    if !saw_scan {
        return Err("JPEG has no image scan".into());
    }
    fs::write(dst, &out).map_err(|e| format!("{}: {e}", dst.display()))?;
    Ok((data.len() - out.len()) as u64)
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}
