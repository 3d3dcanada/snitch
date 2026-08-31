//! PNG chunk surgery: what to keep, what to drop, and what the text chunks are saying.
//!
//! STRIPPING IS CHUNK FILTERING, NOT RE-ENCODING. A PNG is a signature followed by length-tagged
//! chunks. Keeping the critical ones and the handful that affect display, and dropping everything
//! else, leaves the compressed image data untouched, so the decoded pixels come out byte-identical.
//! Re-encoding to "clear" metadata throws away nothing visible but costs time and risks a decoder
//! disagreement, and this tool will not do it.

use std::fs;
use std::io::Read;
use std::path::Path;

use flate2::read::ZlibDecoder;

pub const SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1a, b'\n'];

/// Ancillary chunks that change how the image looks. Everything else ancillary is metadata and
/// goes, including `caBX` (C2PA), `eXIf`, `tEXt`, `zTXt` and `iTXt`.
const DISPLAY_CHUNKS: [&[u8; 4]; 15] = [
    b"PLTE", b"tRNS", b"gAMA", b"cHRM", b"sRGB", b"iCCP", b"cICP", b"mDCv", b"cLLi", b"sBIT",
    b"pHYs", b"bKGD", b"acTL", b"fcTL", b"fdAT",
];

const TEXT_CHUNKS: [&[u8; 4]; 3] = [b"tEXt", b"zTXt", b"iTXt"];

/// Keywords that image generators write. Matching the keyword rather than the text is deliberate:
/// a photograph whose caption happens to mention Stable Diffusion is still a photograph.
pub const GENERATOR_KEYWORDS: [&str; 9] = [
    "parameters",  // Stable Diffusion WebUI: AUTOMATIC1111, Forge
    "prompt",      // ComfyUI
    "workflow",    // ComfyUI
    "dream",       // early Stable Diffusion
    "sd-metadata", // InvokeAI
    "invokeai_metadata",
    "invokeai_graph",
    "aigenerated",
    "generation_data",
];

/// One decoded text chunk.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct TextChunk {
    pub chunk: String,
    pub keyword: String,
    pub text: String,
    /// True when the chunk was longer than this tool is willing to hold in memory. Serialised only
    /// when true, so an ordinary file's report is byte-identical to what it was before the cap.
    #[serde(skip_serializing_if = "is_false", default)]
    pub truncated: bool,
}

fn is_false(b: &bool) -> bool {
    !*b
}

impl TextChunk {
    pub fn is_generator(&self) -> bool {
        let lower = self.keyword.to_ascii_lowercase();
        GENERATOR_KEYWORDS.contains(&lower.as_str())
    }
}

// CRC-32 is twenty-five lines and one table. A crate for it would be a dependency that saves
// nothing, which is the bar the house style sets.
const fn crc_table() -> [u32; 256] {
    let mut table = [0u32; 256];
    let mut n = 0;
    while n < 256 {
        let mut c = n as u32;
        let mut k = 0;
        while k < 8 {
            c = if c & 1 != 0 {
                0xEDB8_8320 ^ (c >> 1)
            } else {
                c >> 1
            };
            k += 1;
        }
        table[n] = c;
        n += 1;
    }
    table
}

static CRC_TABLE: [u32; 256] = crc_table();

pub fn crc32(bytes: &[u8]) -> u32 {
    let mut c = 0xFFFF_FFFFu32;
    for &b in bytes {
        c = CRC_TABLE[((c ^ b as u32) & 0xFF) as usize] ^ (c >> 8);
    }
    c ^ 0xFFFF_FFFF
}

pub fn chunk(ctype: &[u8; 4], payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(12 + payload.len());
    out.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    out.extend_from_slice(ctype);
    out.extend_from_slice(payload);
    let mut crc_input = Vec::with_capacity(4 + payload.len());
    crc_input.extend_from_slice(ctype);
    crc_input.extend_from_slice(payload);
    out.extend_from_slice(&crc32(&crc_input).to_be_bytes());
    out
}

fn be_u32(bytes: &[u8]) -> u32 {
    u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

/// The most any one text chunk is allowed to inflate to, and the most all of them together are.
///
/// MEASURED, NOT GUESSED. A 510 KB PNG carrying one zTXt chunk of compressed zeros drove `snitch`
/// to 3.6 GB resident and eight seconds of work, because the inflated bytes are then copied for
/// UTF-8 handling and again for JSON escaping. A 4 MB file holding 200,000 tiny chunks reached
/// 428 MB. Neither is a legitimate file: the largest real generator payload anyone writes is a
/// ComfyUI workflow, and those are tens of kilobytes. So both are capped, and a chunk that hits
/// the cap is reported as truncated rather than silently shortened.
const MAX_CHUNK_TEXT: usize = 1 << 20; // 1 MB
const MAX_TEXT_BUDGET: usize = 4 << 20; // 4 MB across the whole file
const MAX_TEXT_CHUNKS: usize = 256;

/// Inflate, refusing to keep going past `MAX_CHUNK_TEXT`. Returns the bytes and whether it stopped
/// early, so the caller can say so instead of quietly handing back a shortened prompt.
fn inflate(data: &[u8]) -> Option<(Vec<u8>, bool)> {
    let mut out = Vec::new();
    // take() bounds what is read out of the decoder, so the allocation is bounded too. Reading one
    // byte past the cap is how we learn there was more, without decompressing all of it.
    ZlibDecoder::new(data)
        .take(MAX_CHUNK_TEXT as u64 + 1)
        .read_to_end(&mut out)
        .ok()?;
    let truncated = out.len() > MAX_CHUNK_TEXT;
    out.truncate(MAX_CHUNK_TEXT);
    Some((out, truncated))
}

/// Latin-1 is a byte-for-byte subset of the first 256 code points, which is what tEXt and zTXt are
/// defined to hold. Decoding it by hand avoids pulling an encoding crate for one line.
fn latin1(bytes: &[u8]) -> String {
    bytes.iter().map(|&b| b as char).collect()
}

fn split_nul(data: &[u8]) -> Option<(&[u8], &[u8])> {
    let at = data.iter().position(|&b| b == 0)?;
    Some((&data[..at], &data[at + 1..]))
}

/// Decode one text chunk payload into (keyword, text), or None if it is malformed.
fn decode_text(ctype: &[u8; 4], payload: &[u8]) -> Option<(String, String, bool)> {
    let (keyword, rest) = split_nul(payload)?;
    let name = latin1(keyword);
    match ctype {
        b"tEXt" => {
            let truncated = rest.len() > MAX_CHUNK_TEXT;
            Some((
                name,
                latin1(&rest[..rest.len().min(MAX_CHUNK_TEXT)]),
                truncated,
            ))
        }
        b"zTXt" => {
            // 0 is the only defined compression method.
            if rest.first() != Some(&0) {
                return None;
            }
            let (bytes, truncated) = inflate(&rest[1..])?;
            Some((name, latin1(&bytes), truncated))
        }
        // iTXt: compression flag, compression method, language tag, translated keyword, then text.
        b"iTXt" => {
            if rest.len() < 2 {
                return None;
            }
            let (compressed, method) = (rest[0], rest[1]);
            let (_language, rest) = split_nul(&rest[2..])?;
            let (_translated, text) = split_nul(rest)?;
            let (bytes, truncated) = if compressed != 0 {
                if method != 0 {
                    return None;
                }
                inflate(text)?
            } else {
                (
                    text[..text.len().min(MAX_CHUNK_TEXT)].to_vec(),
                    text.len() > MAX_CHUNK_TEXT,
                )
            };
            Some((
                name,
                String::from_utf8_lossy(&bytes).into_owned(),
                truncated,
            ))
        }
        _ => None,
    }
}

/// Every tEXt, zTXt and iTXt chunk in a PNG, decoded, in file order. Empty for anything else.
///
/// ExifTool folds these into its PNG group beside IHDR fields, with nothing to distinguish a
/// generator's prompt from an image dimension. Reading the container directly keeps the chunk type,
/// which is the part worth telling someone about.
///
/// Reporting is not surgery: this walker stops at the first inconsistency and returns what it read,
/// so a damaged file still reports what could be recovered. `strip` stays strict, and is the one
/// that refuses.
pub fn read_text(path: &Path) -> Vec<TextChunk> {
    let Ok(data) = fs::read(path) else {
        return Vec::new();
    };
    if data.len() < 8 || data[..8] != SIGNATURE {
        return Vec::new();
    }
    let mut found = Vec::new();
    let mut spent = 0usize;
    let mut i = 8usize;
    while i + 12 <= data.len() {
        let length = be_u32(&data[i..i + 4]) as usize;
        let ctype: [u8; 4] = data[i + 4..i + 8].try_into().expect("four bytes");
        let Some(total) = length.checked_add(12) else {
            break;
        };
        if total > data.len() - i {
            break;
        }
        if TEXT_CHUNKS.contains(&&ctype) && found.len() < MAX_TEXT_CHUNKS && spent < MAX_TEXT_BUDGET
        {
            if let Some((keyword, text, truncated)) =
                decode_text(&ctype, &data[i + 8..i + 8 + length])
            {
                spent += text.len();
                found.push(TextChunk {
                    chunk: String::from_utf8_lossy(&ctype).into_owned(),
                    keyword,
                    text,
                    truncated,
                });
            }
        }
        i += total;
        if &ctype == b"IEND" {
            break;
        }
    }
    found
}

/// Keep image and display chunks, drop private metadata including C2PA's caBX chunk.
///
/// Returns the number of bytes removed. `orientation` is an optional raw EXIF payload to reinsert
/// as an `eXIf` chunk, so a rotation the viewer depends on survives the strip.
pub fn strip(src: &Path, dst: &Path, orientation: Option<&[u8]>) -> Result<u64, String> {
    let data = fs::read(src).map_err(|e| format!("{}: {e}", src.display()))?;
    if data.len() < 8 || data[..8] != SIGNATURE {
        return Err("not a PNG".into());
    }
    let orientation_chunk = orientation.map(|payload| {
        // Pillow writes the segment with an "Exif\0\0" preamble that belongs to JPEG APP1, not to
        // a PNG eXIf chunk, so it comes off here exactly as the Python did.
        let body = payload.strip_prefix(b"Exif\x00\x00").unwrap_or(payload);
        chunk(b"eXIf", body)
    });

    let mut out = Vec::with_capacity(data.len());
    out.extend_from_slice(&SIGNATURE);
    let mut i = 8usize;
    let mut saw_header = false;
    let mut saw_end = false;
    let mut inserted_orientation = false;

    while i < data.len() {
        if i + 12 > data.len() {
            return Err("truncated PNG chunk".into());
        }
        let length = be_u32(&data[i..i + 4]) as usize;
        let ctype: [u8; 4] = data[i + 4..i + 8].try_into().expect("four bytes");
        // Bounded rather than `length + 12`, to match read_text. `length` is a u32 out of the
        // file: on a 64-bit usize that add cannot overflow, but on a 32-bit one it wraps to a
        // small number in release, the length check below then passes, and the slice two lines
        // further down panics. Nothing ships a 32-bit target today, and `cargo install` on an
        // armv7 or i686 machine builds one.
        let Some(total) = length.checked_add(12) else {
            return Err("PNG chunk length is not a length".into());
        };
        if total > data.len() - i {
            return Err("truncated PNG chunk payload".into());
        }
        let payload = &data[i + 8..i + 8 + length];
        let expected = be_u32(&data[i + 8 + length..i + total]);
        let mut crc_input = Vec::with_capacity(4 + length);
        crc_input.extend_from_slice(&ctype);
        crc_input.extend_from_slice(payload);
        if expected != crc32(&crc_input) {
            return Err(format!(
                "bad PNG CRC in {} chunk",
                String::from_utf8_lossy(&ctype)
            ));
        }
        if !saw_header && &ctype != b"IHDR" {
            return Err("PNG does not start with IHDR".into());
        }
        if &ctype == b"IHDR" {
            if saw_header {
                return Err("PNG has more than one IHDR chunk".into());
            }
            saw_header = true;
        }
        if &ctype == b"IDAT" && !inserted_orientation {
            if let Some(ref oc) = orientation_chunk {
                out.extend_from_slice(oc);
                inserted_orientation = true;
            }
        }
        // PNG says an uppercase first letter means the chunk is critical to decoding.
        let is_critical = ctype[0].is_ascii_uppercase();
        if is_critical || DISPLAY_CHUNKS.contains(&&ctype) {
            out.extend_from_slice(&data[i..i + total]);
        }
        i += total;
        if &ctype == b"IEND" {
            saw_end = true;
            break;
        }
    }
    if !saw_header || !saw_end {
        return Err("PNG is missing a required terminal chunk".into());
    }
    if i != data.len() {
        return Err("PNG has data after IEND".into());
    }
    fs::write(dst, &out).map_err(|e| format!("{}: {e}", dst.display()))?;
    Ok((data.len() - out.len()) as u64)
}
