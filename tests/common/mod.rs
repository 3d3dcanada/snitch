//! Fixtures shared by the integration tests. Nothing here touches the network or the user's home.

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::Command;

use image::{ImageBuffer, Rgb, RgbImage};

pub fn have(tool: &str) -> bool {
    snitch::exif::which(tool).is_some()
}

/// A JPEG with nothing in it but pixels.
pub fn plain_jpeg(path: &Path, w: u32, h: u32) {
    let image: RgbImage =
        ImageBuffer::from_fn(w, h, |x, y| Rgb([(x % 256) as u8, (y % 256) as u8, 140]));
    image.save(path).expect("write jpeg");
}

pub fn plain_png(path: &Path, w: u32, h: u32) {
    let image: RgbImage =
        ImageBuffer::from_fn(w, h, |x, y| Rgb([(x % 256) as u8, 90, (y % 256) as u8]));
    image.save(path).expect("write png");
}

/// One text chunk payload: `keyword\0text`, which is the tEXt layout.
pub fn text_payload(keyword: &str, text: &str) -> Vec<u8> {
    let mut out = keyword.as_bytes().to_vec();
    out.push(0);
    out.extend_from_slice(text.as_bytes());
    out
}

/// A PNG shaped like one that comes out of ComfyUI or the Stable Diffusion WebUI.
pub fn generator_png(path: &Path) {
    plain_png(path, 24, 18);
    insert_chunk(
        path,
        &snitch::png::chunk(
            b"tEXt",
            &text_payload(
                "parameters",
                "a cat, (ugly hands:1.4)\nSteps: 30, Sampler: Euler a",
            ),
        ),
    );
    insert_chunk(
        path,
        &snitch::png::chunk(
            b"tEXt",
            &text_payload("workflow", r#"{"nodes":[{"id":1,"type":"KSampler"}]}"#),
        ),
    );
}

/// Insert a raw chunk just before IEND, so the encoder does not get a say in the bytes.
pub fn insert_chunk(path: &Path, chunk: &[u8]) {
    let data = std::fs::read(path).expect("read png");
    let at = find_last(&data, b"IEND").expect("IEND") - 4;
    let mut out = Vec::with_capacity(data.len() + chunk.len());
    out.extend_from_slice(&data[..at]);
    out.extend_from_slice(chunk);
    out.extend_from_slice(&data[at..]);
    std::fs::write(path, out).expect("write png");
}

fn find_last(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).rposition(|w| w == needle)
}

/// Write EXIF, GPS, IPTC and XMP into a file with ExifTool. Skipped by callers when it is absent.
pub fn write_metadata(path: &Path, args: &[&str]) {
    let out = Command::new("exiftool")
        .args(["-overwrite_original", "-q"])
        .args(args)
        .arg(path)
        .output()
        .expect("run exiftool");
    assert!(
        out.status.success(),
        "exiftool failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

pub fn read_tag(path: &Path, tag: &str) -> String {
    let out = Command::new("exiftool")
        .args(["-s", "-s", "-s", tag])
        .arg(path)
        .output()
        .expect("run exiftool");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// A directory that cleans itself up, so a failing test does not leave litter beside the repo.
pub struct TempDir(pub PathBuf);

impl TempDir {
    pub fn new(label: &str) -> TempDir {
        let base = std::env::temp_dir().join(format!(
            "snitch-test-{}-{}-{:?}",
            label,
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir_all(&base).expect("create temp dir");
        TempDir(base)
    }

    pub fn path(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
