//! Composite a visible mark into the pixels.
//!
//! THIS RE-ENCODES. It is the only operation here that costs image quality, and the CLI says so.
//! Quality defaults to 94, which is visually lossless for a photograph at normal viewing sizes.
//!
//! A visible mark survives a screenshot because it is part of the pixels, but platforms may crop,
//! resize or soften it.
//!
//! THE RE-ENCODE MUST CARRY THE METADATA ACROSS, and this is not a nicety. The encoder writes a
//! fresh file, so without this the stamp silently destroys the camera block, the colour profile
//! and the GPS. That made `credit --stamp --keep-gps` a lie: it kept nothing, because the stamp
//! had already thrown the EXIF away before the metadata step ran. Found by using the tool rather
//! than by testing its parts, and the copy-back below is the fix.

use std::path::{Path, PathBuf};
use std::process::Command;

use ab_glyph::{Font, FontVec, PxScale, ScaleFont};
use image::{DynamicImage, Rgba, RgbaImage};

pub const CORNERS: [&str; 4] = ["bottom-right", "bottom-left", "top-right", "top-left"];

/// Where a usable font tends to live. The Python shipped Pillow's bundled face; embedding a TTF
/// here would add most of a megabyte to every binary, so the font is found at runtime and the
/// error names the fix when it cannot be.
const FONT_CANDIDATES: [&str; 10] = [
    "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf",
    "/usr/share/fonts/truetype/liberation/LiberationSans-Bold.ttf",
    "/usr/share/fonts/TTF/DejaVuSans-Bold.ttf",
    "/usr/share/fonts/dejavu/DejaVuSans-Bold.ttf",
    "/usr/share/fonts/liberation-sans/LiberationSans-Bold.ttf",
    "/System/Library/Fonts/Supplemental/Arial Bold.ttf",
    "/System/Library/Fonts/Helvetica.ttc",
    "C:\\Windows\\Fonts\\arialbd.ttf",
    "C:\\Windows\\Fonts\\segoeuib.ttf",
    "C:\\Windows\\Fonts\\arial.ttf",
];

pub struct Options<'a> {
    pub text: &'a str,
    pub subtext: Option<&'a str>,
    pub logo: Option<&'a Path>,
    pub corner: &'a str,
    pub scale: f32,
    pub opacity: f32,
    pub font: Option<&'a Path>,
    pub quality: u8,
}

impl Default for Options<'_> {
    fn default() -> Self {
        Options {
            text: "",
            subtext: None,
            logo: None,
            corner: "bottom-right",
            scale: 0.05,
            opacity: 0.85,
            font: None,
            quality: 94,
        }
    }
}

fn load_font(explicit: Option<&Path>) -> Result<FontVec, String> {
    if let Some(path) = explicit {
        let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
        return FontVec::try_from_vec(bytes)
            .map_err(|_| format!("{}: not a font this tool can read", path.display()));
    }
    for candidate in FONT_CANDIDATES {
        if let Ok(bytes) = std::fs::read(candidate) {
            if let Ok(font) = FontVec::try_from_vec(bytes) {
                return Ok(font);
            }
        }
    }
    Err("no system font found for the stamp text.\n  \
         Point at one yourself:  --font /path/to/font.ttf\n  \
         Debian/Ubuntu:          sudo apt install fonts-dejavu-core"
        .into())
}

fn text_width(font: &FontVec, scale: PxScale, text: &str) -> f32 {
    let scaled = font.as_scaled(scale);
    let mut width = 0.0;
    let mut previous: Option<char> = None;
    for c in text.chars() {
        let id = font.glyph_id(c);
        width += scaled.h_advance(id);
        if let Some(prev) = previous {
            width += scaled.kern(font.glyph_id(prev), id);
        }
        previous = Some(c);
    }
    width
}

fn draw_text(
    canvas: &mut RgbaImage,
    font: &FontVec,
    scale: PxScale,
    x: f32,
    y: f32,
    text: &str,
    alpha: u8,
) {
    let scaled = font.as_scaled(scale);
    let mut caret = x;
    let baseline = y + scaled.ascent();
    let mut previous: Option<char> = None;
    for c in text.chars() {
        let id = font.glyph_id(c);
        if let Some(prev) = previous {
            caret += scaled.kern(font.glyph_id(prev), id);
        }
        let glyph = id.with_scale_and_position(scale, ab_glyph::point(caret, baseline));
        if let Some(outline) = font.outline_glyph(glyph) {
            let bounds = outline.px_bounds();
            outline.draw(|gx, gy, coverage| {
                let px = bounds.min.x as i32 + gx as i32;
                let py = bounds.min.y as i32 + gy as i32;
                if px < 0 || py < 0 {
                    return;
                }
                let (px, py) = (px as u32, py as u32);
                if px >= canvas.width() || py >= canvas.height() {
                    return;
                }
                let a = (coverage * alpha as f32) as u8;
                if a == 0 {
                    return;
                }
                blend(canvas, px, py, Rgba([255, 255, 255, a]));
            });
        }
        caret += scaled.h_advance(id);
        previous = Some(c);
    }
}

fn blend(canvas: &mut RgbaImage, x: u32, y: u32, top: Rgba<u8>) {
    let under = *canvas.get_pixel(x, y);
    let ta = top[3] as f32 / 255.0;
    let ua = under[3] as f32 / 255.0;
    let out_a = ta + ua * (1.0 - ta);
    if out_a <= 0.0 {
        canvas.put_pixel(x, y, Rgba([0, 0, 0, 0]));
        return;
    }
    let mut out = [0u8; 4];
    for i in 0..3 {
        let c = (top[i] as f32 * ta + under[i] as f32 * ua * (1.0 - ta)) / out_a;
        out[i] = c.round().clamp(0.0, 255.0) as u8;
    }
    out[3] = (out_a * 255.0).round().clamp(0.0, 255.0) as u8;
    canvas.put_pixel(x, y, Rgba(out));
}

fn rounded_rect(canvas: &mut RgbaImage, radius: i32, fill: Rgba<u8>) {
    let (w, h) = (canvas.width() as i32, canvas.height() as i32);
    for y in 0..h {
        for x in 0..w {
            // Inside the straight part, or inside one of the four corner discs.
            let cx = if x < radius {
                radius
            } else if x >= w - radius {
                w - 1 - radius
            } else {
                x
            };
            let cy = if y < radius {
                radius
            } else if y >= h - radius {
                h - 1 - radius
            } else {
                y
            };
            let (dx, dy) = ((x - cx) as f32, (y - cy) as f32);
            if dx * dx + dy * dy <= (radius as f32) * (radius as f32) {
                canvas.put_pixel(x as u32, y as u32, fill);
            }
        }
    }
}

/// Draw the mark and write `dst`. Returns the path written.
pub fn stamp(src: &Path, dst: &Path, options: &Options) -> Result<PathBuf, String> {
    if !CORNERS.contains(&options.corner) {
        return Err(format!("corner must be one of {}", CORNERS.join(", ")));
    }
    if !(options.scale > 0.0 && options.scale <= 1.0) {
        return Err("scale must be greater than 0 and at most 1".into());
    }
    if !(options.opacity > 0.0 && options.opacity <= 1.0) {
        return Err("opacity must be greater than 0 and at most 1".into());
    }
    if !(1..=100).contains(&options.quality) {
        return Err("quality must be between 1 and 100".into());
    }
    let ext = dst
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if !matches!(ext.as_str(), "jpg" | "jpeg" | "png") {
        return Err("visible stamping supports JPEG and PNG only".into());
    }
    if options.text.is_empty() && options.logo.is_none() {
        return Err("a visible stamp needs text or a logo".into());
    }
    if let Some(logo) = options.logo {
        if !logo.is_file() {
            return Err(format!("logo not found: {}", logo.display()));
        }
    }

    let source = image::open(src).map_err(|e| format!("{}: {e}", src.display()))?;
    let source_has_alpha = source.color().has_alpha();
    let mut canvas = source.to_rgba8();
    let (w, h) = (canvas.width(), canvas.height());

    let unit = (w.min(h) as f32 * options.scale).max(20.0) as u32;
    let pad = ((unit as f32 * 0.30) as u32).max(6);
    let text_px = unit as f32 * 0.50;
    let sub_px = unit as f32 * 0.30;
    let font = load_font(options.font)?;
    let main_scale = PxScale::from(text_px);
    let sub_scale = PxScale::from(sub_px);

    let logo_image = match options.logo {
        Some(path) => {
            let logo = image::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
            let ratio = unit as f32 / logo.height().max(1) as f32;
            let width = ((logo.width() as f32 * ratio) as u32).max(1);
            Some(
                logo.resize_exact(width, unit, image::imageops::FilterType::Lanczos3)
                    .to_rgba8(),
            )
        }
        None => None,
    };

    let tw = text_width(&font, main_scale, options.text);
    let sw = options
        .subtext
        .map_or(0.0, |s| text_width(&font, sub_scale, s));
    let text_w = tw.max(sw) as u32;
    let gap = if logo_image.is_some() {
        (unit as f32 * 0.30) as u32
    } else {
        0
    };
    let lw = logo_image.as_ref().map_or(0, |l| l.width());

    let plate_w = pad * 2 + lw + gap + text_w;
    let plate_h = pad * 2 + unit;
    let mut plate = RgbaImage::from_pixel(plate_w.max(1), plate_h.max(1), Rgba([0, 0, 0, 0]));
    rounded_rect(
        &mut plate,
        (plate_h / 4) as i32,
        Rgba([0, 0, 0, (255.0 * 0.42) as u8]),
    );

    if let Some(logo) = &logo_image {
        for (lx, ly, px) in logo.enumerate_pixels() {
            let (x, y) = (pad + lx, pad + ly);
            if x < plate.width() && y < plate.height() {
                blend(&mut plate, x, y, *px);
            }
        }
    }
    let tx = (pad + lw + gap) as f32;
    match options.subtext {
        Some(sub) => {
            draw_text(
                &mut plate,
                &font,
                main_scale,
                tx,
                pad as f32 + unit as f32 * 0.06,
                options.text,
                255,
            );
            draw_text(
                &mut plate,
                &font,
                sub_scale,
                tx,
                pad as f32 + unit as f32 * 0.56,
                sub,
                190,
            );
        }
        None => draw_text(
            &mut plate,
            &font,
            main_scale,
            tx,
            pad as f32 + unit as f32 * 0.26,
            options.text,
            255,
        ),
    }

    if options.opacity < 1.0 {
        for px in plate.pixels_mut() {
            px[3] = (px[3] as f32 * options.opacity) as u8;
        }
    }

    let inset = ((w.min(h) as f32 * 0.022) as u32).max(8);
    let available_w = w.saturating_sub(2 * inset).max(1);
    let available_h = h.saturating_sub(2 * inset).max(1);
    let fit = 1.0f32
        .min(available_w as f32 / plate.width() as f32)
        .min(available_h as f32 / plate.height() as f32);
    let plate = if fit < 1.0 {
        DynamicImage::ImageRgba8(plate)
            .resize_exact(
                ((plate_w as f32 * fit) as u32).max(1),
                ((plate_h as f32 * fit) as u32).max(1),
                image::imageops::FilterType::Lanczos3,
            )
            .to_rgba8()
    } else {
        plate
    };

    let x = if options.corner.contains("right") {
        w.saturating_sub(plate.width() + inset)
    } else {
        inset
    };
    let y = if options.corner.contains("bottom") {
        h.saturating_sub(plate.height() + inset)
    } else {
        inset
    };
    for (px, py, pixel) in plate.enumerate_pixels() {
        let (cx, cy) = (x + px, y + py);
        if cx < w && cy < h {
            blend(&mut canvas, cx, cy, *pixel);
        }
    }

    if ext == "png" {
        if source_has_alpha {
            canvas
                .save(dst)
                .map_err(|e| format!("{}: {e}", dst.display()))?;
        } else {
            DynamicImage::ImageRgba8(canvas)
                .to_rgb8()
                .save(dst)
                .map_err(|e| format!("{}: {e}", dst.display()))?;
        }
    } else {
        let rgb = DynamicImage::ImageRgba8(canvas).to_rgb8();
        let file = std::fs::File::create(dst).map_err(|e| format!("{}: {e}", dst.display()))?;
        let mut writer = std::io::BufWriter::new(file);
        let mut encoder =
            image::codecs::jpeg::JpegEncoder::new_with_quality(&mut writer, options.quality);
        encoder
            .encode(
                rgb.as_raw(),
                rgb.width(),
                rgb.height(),
                image::ExtendedColorType::Rgb8,
            )
            .map_err(|e| format!("{}: {e}", dst.display()))?;
    }

    carry_metadata(src, dst);
    Ok(dst.to_path_buf())
}

/// Copy the source's metadata onto the re-encoded file. Failure is reported, never silent, but it
/// does not undo the stamp: the user has a stamped image either way and deserves to know if the
/// camera block did not come with it.
fn carry_metadata(src: &Path, dst: &Path) {
    if crate::exif::which("exiftool").is_none() {
        eprintln!(
            "  exiftool is not installed, so the stamped copy carries no EXIF, ICC or GPS from \
             the original."
        );
        return;
    }
    let out = Command::new("exiftool")
        .args(["-TagsFromFile"])
        .arg(src)
        .args(["-all:all", "-overwrite_original", "-q", "-P"])
        .arg(dst)
        .output();
    match out {
        Ok(out) if out.status.success() => {}
        Ok(out) => eprintln!(
            "  the stamp was written but its metadata did not copy across: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ),
        Err(e) => eprintln!("  the stamp was written but exiftool could not run: {e}"),
    }
}
