//! Image preprocessing and vision/classification tools (ImagePreprocess, Vision).
//!
//! Shared infrastructure lives in the parent module (reached via `use super::*`).

use super::*;

#[derive(Debug, Default, Clone, Copy)]
pub struct ImagePreprocessTool;

#[derive(Debug, Default, Clone, Copy)]
pub struct VisionTool;

#[derive(Debug, Clone, PartialEq)]
struct ImageAnalysis {
    width: u32,
    height: u32,
    mean_luma: f64,
    luma_stddev: f64,
    edge_luma: f64,
    content_box: Option<NormalizedRect>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct NormalizedRect {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

impl Tool for ImagePreprocessTool {
    fn name(&self) -> &'static str {
        "ImagePreprocess"
    }

    fn description(&self) -> &'static str {
        "Inspect, crop, resize, and enhance local images before visual analysis."
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn call(&self, input: serde_json::Value, context: &ToolContext) -> ToolResult {
        let Some(file_path) = input.get("file_path").and_then(serde_json::Value::as_str) else {
            return ToolResult::error("file_path is required");
        };
        if file_path.trim().is_empty() {
            return ToolResult::error("file_path is required");
        }
        let Some(operation) = input.get("operation").and_then(serde_json::Value::as_str) else {
            return ToolResult::error("operation must be one of: inspect, preprocess");
        };
        if !matches!(operation, "inspect" | "preprocess") {
            return ToolResult::error("operation must be one of: inspect, preprocess");
        }

        let path = context.resolve_path(file_path);
        if !path.exists() {
            return ToolResult::error(format!("File not found: {}", path.display()));
        }
        let image = match image::open(&path) {
            Ok(image) => image,
            Err(error) => {
                return ToolResult::error(format!(
                    "Failed to load image: {}: {error}",
                    path.display()
                ));
            }
        };

        match operation {
            "inspect" => ToolResult::text(render_image_inspect(&image, &path)),
            "preprocess" => match preprocess_image(&image, &path, &input) {
                Ok(report) => ToolResult::text(report),
                Err(error) => ToolResult::error(error),
            },
            _ => unreachable!("validated operation"),
        }
    }
}

impl Tool for VisionTool {
    fn name(&self) -> &'static str {
        "Vision"
    }

    fn description(&self) -> &'static str {
        "Analyze local images and PDFs with OCR, layout extraction, or classification."
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn call(&self, input: serde_json::Value, context: &ToolContext) -> ToolResult {
        let Some(file_path) = input.get("file_path").and_then(serde_json::Value::as_str) else {
            return ToolResult::error("file_path is required");
        };
        if file_path.trim().is_empty() {
            return ToolResult::error("file_path is required");
        }
        let Some(operation) = input.get("operation").and_then(serde_json::Value::as_str) else {
            return ToolResult::error("operation must be one of: ocr, layout, classify");
        };
        if !matches!(operation, "ocr" | "layout" | "classify") {
            return ToolResult::error("operation must be one of: ocr, layout, classify");
        }

        let language_hints = match parse_vision_language_hints(&input) {
            Ok(language_hints) => language_hints,
            Err(error) => return ToolResult::error(error),
        };
        let min_confidence = match parse_vision_min_confidence(&input) {
            Ok(min_confidence) => min_confidence,
            Err(error) => return ToolResult::error(error),
        };

        let path = context.resolve_path(file_path);
        if !path.exists() {
            return ToolResult::error(format!("File not found: {}", path.display()));
        }
        if !path.is_file() {
            return ToolResult::error(format!("Path is not a file: {}", path.display()));
        }

        if is_pdf_path(&path) {
            return match operation {
                "classify" => ToolResult::text(render_pdf_classification(&path)),
                "ocr" | "layout" => match run_pdftotext_vision(&path, operation, &input) {
                    Ok(output) => ToolResult::text(output),
                    Err(error) => ToolResult::error(error),
                },
                _ => unreachable!("validated operation"),
            };
        }

        let image = match image::open(&path) {
            Ok(image) => image,
            Err(error) => {
                return ToolResult::error(format!(
                    "Failed to load image: {}: {error}",
                    path.display()
                ));
            }
        };

        match operation {
            "classify" => ToolResult::text(render_vision_classification(&image, &path)),
            "ocr" => match run_tesseract_ocr(&path, &language_hints) {
                Ok(output) => ToolResult::text(output),
                Err(error) => ToolResult::error(error),
            },
            "layout" => {
                match run_tesseract_layout(&path, &image, &language_hints, min_confidence) {
                    Ok(output) => ToolResult::text(output),
                    Err(error) => ToolResult::error(error),
                }
            }
            _ => unreachable!("validated operation"),
        }
    }
}

impl ImageAnalysis {
    fn is_likely_blank(&self) -> bool {
        self.luma_stddev < 2.0
    }

    fn content_box_description(&self) -> String {
        self.content_box.map_or_else(
            || String::from("none"),
            |rect| {
                format!(
                    "x={:.3} y={:.3} width={:.3} height={:.3}",
                    rect.x, rect.y, rect.width, rect.height
                )
            },
        )
    }
}

fn render_image_inspect(image: &DynamicImage, path: &Path) -> String {
    let analysis = analyse_image(image, 192);
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("image");
    format!(
        "[ImagePreprocess.inspect] {name}\nsize: {}x{}\nmean_luma: {:.1}\nluma_stddev: {:.1}\nedge_luma: {:.1}\nlikely_blank: {}\ncontent_box: {}",
        analysis.width,
        analysis.height,
        analysis.mean_luma,
        analysis.luma_stddev,
        analysis.edge_luma,
        analysis.is_likely_blank(),
        analysis.content_box_description()
    )
}

fn preprocess_image(
    image: &DynamicImage,
    path: &Path,
    input: &serde_json::Value,
) -> Result<String, String> {
    let original_analysis = analyse_image(image, 192);
    let mut processed = image.to_rgba8();
    let mut steps = Vec::new();

    if input
        .get("auto_trim")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
        && let Some(rect) = original_analysis.content_box
    {
        processed = crop_normalized(&processed, rect);
        steps.push(String::from("auto_trim"));
    }

    if let Some(rect) = parse_normalized_crop(input.get("crop")) {
        processed = crop_normalized(&processed, rect);
        steps.push(String::from("crop"));
    }

    if input
        .get("enhance_text")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
    {
        processed = enhance_text_image(&processed);
        steps.push(String::from("enhance_text"));
    }

    let max_dimension = input
        .get("max_dimension")
        .and_then(serde_json::Value::as_u64)
        .and_then(|value| u32::try_from(value).ok())
        .unwrap_or(1600)
        .clamp(256, 4096);
    let longest_side = processed.width().max(processed.height());
    if longest_side > max_dimension {
        let scale = f64::from(max_dimension) / f64::from(longest_side);
        let width = (f64::from(processed.width()) * scale).round().max(1.0) as u32;
        let height = (f64::from(processed.height()) * scale).round().max(1.0) as u32;
        processed = imageops::resize(&processed, width, height, FilterType::Lanczos3);
        steps.push(format!("resize_{max_dimension}"));
    }

    let format = input
        .get("format")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("jpeg")
        .to_ascii_lowercase();
    let format = if format == "png" { "png" } else { "jpeg" };
    let encoded = encode_processed_image(&processed, format)?;
    let media_type = if format == "png" {
        "image/png"
    } else {
        "image/jpeg"
    };
    let final_analysis = analyse_rgba_image(&processed, 192);
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("image");
    let encoded_base64 = base64_encode(&encoded);

    Ok(format!(
        "[ImagePreprocess.preprocess] {name}\nsteps: {}\ninput_size: {}x{}\noutput_size: {}x{}\nlikely_blank: {}\ncontent_box: {}\nformat: {media_type}\nimage_base64: {encoded_base64}",
        if steps.is_empty() {
            String::from("none")
        } else {
            steps.join(", ")
        },
        original_analysis.width,
        original_analysis.height,
        final_analysis.width,
        final_analysis.height,
        final_analysis.is_likely_blank(),
        final_analysis.content_box_description()
    ))
}

fn parse_vision_language_hints(input: &serde_json::Value) -> Result<Vec<String>, String> {
    let Some(value) = input.get("language_hints") else {
        return Ok(Vec::new());
    };
    let Some(values) = value.as_array() else {
        return Err(String::from("language_hints must be an array of strings"));
    };
    values
        .iter()
        .map(|value| {
            value
                .as_str()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned)
                .ok_or_else(|| String::from("language_hints must be an array of strings"))
        })
        .collect()
}

fn parse_vision_min_confidence(input: &serde_json::Value) -> Result<f64, String> {
    let Some(value) = input.get("min_confidence") else {
        return Ok(0.5);
    };
    let Some(confidence) = value.as_f64() else {
        return Err(String::from(
            "min_confidence must be a number from 0.0 to 1.0",
        ));
    };
    if !(0.0..=1.0).contains(&confidence) {
        return Err(String::from(
            "min_confidence must be a number from 0.0 to 1.0",
        ));
    }
    Ok(confidence)
}

fn is_pdf_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("pdf"))
}

fn render_vision_classification(image: &DynamicImage, path: &Path) -> String {
    let analysis = analyse_image(image, 192);
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("image");
    let aspect_ratio = f64::from(analysis.width) / f64::from(analysis.height.max(1));
    let mut labels = vec![String::from("image")];
    if analysis.is_likely_blank() {
        labels.push(String::from("blank"));
    } else if analysis.luma_stddev > 40.0 && analysis.edge_luma > 35.0 {
        labels.push(String::from("document-like"));
    } else if analysis.edge_luma > 25.0 {
        labels.push(String::from("screenshot-like"));
    } else {
        labels.push(String::from("photo-like"));
    }
    if aspect_ratio > 1.25 {
        labels.push(String::from("landscape"));
    } else if aspect_ratio < 0.8 {
        labels.push(String::from("portrait"));
    }

    format!(
        "[Vision.classify] {name}\nsize: {}x{}\naspect_ratio: {:.2}\nlabels: {}\nmean_luma: {:.1}\nluma_stddev: {:.1}\nedge_luma: {:.1}\nlikely_blank: {}\ncontent_box: {}",
        analysis.width,
        analysis.height,
        aspect_ratio,
        labels.join(", "),
        analysis.mean_luma,
        analysis.luma_stddev,
        analysis.edge_luma,
        analysis.is_likely_blank(),
        analysis.content_box_description()
    )
}

fn render_pdf_classification(path: &Path) -> String {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("document.pdf");
    let size = fs::metadata(path)
        .map(|metadata| metadata.len())
        .unwrap_or(0);
    format!("[Vision.classify] {name}\nlabels: pdf, document\nsize_bytes: {size}")
}

fn run_tesseract_ocr(path: &Path, language_hints: &[String]) -> Result<String, String> {
    let output = run_tesseract(path, language_hints, None)?;
    let text = output.trim();
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("image");
    Ok(format!(
        "[Vision.ocr] {name}\n{}",
        if text.is_empty() {
            "[No text detected]"
        } else {
            text
        }
    ))
}

fn run_tesseract_layout(
    path: &Path,
    image: &DynamicImage,
    language_hints: &[String],
    min_confidence: f64,
) -> Result<String, String> {
    let output = run_tesseract(path, language_hints, Some("tsv"))?;
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("image");
    let (width, height) = (
        f64::from(image.width().max(1)),
        f64::from(image.height().max(1)),
    );
    let min_confidence = min_confidence * 100.0;
    let mut lines = vec![format!("[Vision.layout] {name}")];
    for line in output.lines().skip(1) {
        let columns: Vec<&str> = line.split('\t').collect();
        if columns.len() < 12 {
            continue;
        }
        let confidence = columns[10].parse::<f64>().unwrap_or(-1.0);
        let text = columns[11..].join("\t").trim().to_owned();
        if text.is_empty() || confidence < min_confidence {
            continue;
        }
        let left = columns[6].parse::<f64>().unwrap_or(0.0) / width;
        let top = columns[7].parse::<f64>().unwrap_or(0.0) / height;
        let word_width = columns[8].parse::<f64>().unwrap_or(0.0) / width;
        let word_height = columns[9].parse::<f64>().unwrap_or(0.0) / height;
        lines.push(format!(
            "conf={:.0} x={left:.3} y={top:.3} width={word_width:.3} height={word_height:.3} text={text}",
            confidence
        ));
    }
    if lines.len() == 1 {
        lines.push(String::from("[No text regions detected]"));
    }
    Ok(lines.join("\n"))
}

fn run_tesseract(
    path: &Path,
    language_hints: &[String],
    output_format: Option<&str>,
) -> Result<String, String> {
    let mut command = Command::new("tesseract");
    command.arg(path).arg("stdout");
    if !language_hints.is_empty() {
        command.arg("-l").arg(language_hints.join("+"));
    }
    if let Some(output_format) = output_format {
        command.arg(output_format);
    }
    let output = command.output().map_err(|error| {
        format!(
            "Vision OCR requires tesseract on this platform. Install tesseract or use ImagePreprocess/classify only: {error}"
        )
    })?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!(
            "Vision OCR failed: {}",
            stderr.trim().if_empty("unknown error")
        ))
    }
}

fn run_pdftotext_vision(
    path: &Path,
    operation: &str,
    input: &serde_json::Value,
) -> Result<String, String> {
    let (first_page, last_page) = parse_vision_pages(input.get("pages"))?;
    let mut command = Command::new("pdftotext");
    if operation == "layout" {
        command.arg("-layout");
    }
    command
        .arg("-f")
        .arg(first_page.to_string())
        .arg("-l")
        .arg(last_page.to_string())
        .arg(path)
        .arg("-");
    let output = command.output().map_err(|error| {
        format!(
            "Vision PDF OCR requires pdftotext (poppler). Install poppler or convert pages to images: {error}"
        )
    })?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "Vision PDF OCR failed: {}",
            stderr.trim().if_empty("unknown error")
        ));
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("document.pdf");
    Ok(format!(
        "[Vision.{operation}] {name}\npages: {first_page}-{last_page}\n{}",
        if text.trim().is_empty() {
            "[No text detected]"
        } else {
            text.trim()
        }
    ))
}

fn parse_vision_pages(value: Option<&serde_json::Value>) -> Result<(usize, usize), String> {
    let Some(value) = value else {
        return Ok((1, 1));
    };
    if let Some(page) = value.as_u64() {
        let page = usize::try_from(page).map_err(|_| String::from("pages is too large"))?;
        if page == 0 {
            return Err(String::from("pages must start at 1"));
        }
        return Ok((page, page));
    }
    let Some(pages) = value
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Err(String::from(
            "pages must be a page number or range like 1-3",
        ));
    };
    if let Some((start, end)) = pages.split_once('-') {
        let start = start
            .trim()
            .parse::<usize>()
            .map_err(|_| String::from("pages must be a page number or range like 1-3"))?;
        let end = end
            .trim()
            .parse::<usize>()
            .map_err(|_| String::from("pages must be a page number or range like 1-3"))?;
        if start == 0 || end < start {
            return Err(String::from(
                "pages must be a page number or range like 1-3",
            ));
        }
        return Ok((start, end.min(start + 4)));
    }
    let page = pages
        .parse::<usize>()
        .map_err(|_| String::from("pages must be a page number or range like 1-3"))?;
    if page == 0 {
        return Err(String::from("pages must start at 1"));
    }
    Ok((page, page))
}

fn analyse_image(image: &DynamicImage, sample_limit: u32) -> ImageAnalysis {
    analyse_rgba_image(&image.to_rgba8(), sample_limit)
}

fn analyse_rgba_image(image: &RgbaImage, sample_limit: u32) -> ImageAnalysis {
    let (width, height) = image.dimensions();
    let longest_side = width.max(height).max(1);
    let scale = (f64::from(sample_limit) / f64::from(longest_side)).min(1.0);
    let sample_width = (f64::from(width) * scale).round().max(1.0) as u32;
    let sample_height = (f64::from(height) * scale).round().max(1.0) as u32;
    let sample = if sample_width == width && sample_height == height {
        image.clone()
    } else {
        imageops::resize(image, sample_width, sample_height, FilterType::Triangle)
    };
    let mut lumas = Vec::with_capacity((sample_width * sample_height) as usize);
    for pixel in sample.pixels() {
        let [r, g, b, _] = pixel.0;
        lumas.push(0.2126 * f64::from(r) + 0.7152 * f64::from(g) + 0.0722 * f64::from(b));
    }
    let count = lumas.len().max(1) as f64;
    let mean_luma = lumas.iter().sum::<f64>() / count;
    let variance = lumas
        .iter()
        .map(|luma| (luma - mean_luma).powi(2))
        .sum::<f64>()
        / count;
    let edge_luma = estimate_edge_luma(&lumas, sample_width as usize, sample_height as usize);
    let content_box = estimate_content_box(
        &lumas,
        sample_width as usize,
        sample_height as usize,
        edge_luma,
    );

    ImageAnalysis {
        width,
        height,
        mean_luma,
        luma_stddev: variance.sqrt(),
        edge_luma,
        content_box,
    }
}

fn estimate_edge_luma(lumas: &[f64], width: usize, height: usize) -> f64 {
    if width == 0 || height == 0 {
        return 0.0;
    }
    let mut samples = Vec::with_capacity(width * 2 + height * 2);
    for x in 0..width {
        samples.push(lumas[x]);
        samples.push(lumas[(height - 1) * width + x]);
    }
    for y in 0..height {
        samples.push(lumas[y * width]);
        samples.push(lumas[y * width + width - 1]);
    }
    samples.iter().sum::<f64>() / samples.len().max(1) as f64
}

fn estimate_content_box(
    lumas: &[f64],
    width: usize,
    height: usize,
    background: f64,
) -> Option<NormalizedRect> {
    let threshold = 10.0_f64.max(32.0_f64.min((background - 127.5).abs() * 0.20));
    let (mut min_x, mut min_y, mut max_x, mut max_y) = (width, height, None, None);
    for y in 0..height {
        for x in 0..width {
            let luma = lumas[y * width + x];
            if (luma - background).abs() > threshold {
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = Some(max_x.map_or(x, |value: usize| value.max(x)));
                max_y = Some(max_y.map_or(y, |value: usize| value.max(y)));
            }
        }
    }
    let (Some(max_x), Some(max_y)) = (max_x, max_y) else {
        return None;
    };
    let min_x = min_x.saturating_sub(2);
    let min_y = min_y.saturating_sub(2);
    let max_x = (max_x + 2).min(width.saturating_sub(1));
    let max_y = (max_y + 2).min(height.saturating_sub(1));
    Some(NormalizedRect {
        x: min_x as f64 / width as f64,
        y: min_y as f64 / height as f64,
        width: (max_x - min_x + 1) as f64 / width as f64,
        height: (max_y - min_y + 1) as f64 / height as f64,
    })
}

fn parse_normalized_crop(value: Option<&serde_json::Value>) -> Option<NormalizedRect> {
    let object = value?.as_object()?;
    let x = object.get("x")?.as_f64()?.clamp(0.0, 1.0);
    let y = object.get("y")?.as_f64()?.clamp(0.0, 1.0);
    let width = object.get("width")?.as_f64()?;
    let height = object.get("height")?.as_f64()?;
    if width <= 0.0 || height <= 0.0 {
        return None;
    }
    Some(NormalizedRect {
        x,
        y,
        width: width.clamp(0.0, 1.0 - x),
        height: height.clamp(0.0, 1.0 - y),
    })
}

fn crop_normalized(image: &RgbaImage, rect: NormalizedRect) -> RgbaImage {
    let x = (rect.x * f64::from(image.width())).floor() as u32;
    let y = (rect.y * f64::from(image.height())).floor() as u32;
    let width = (rect.width * f64::from(image.width())).ceil().max(1.0) as u32;
    let height = (rect.height * f64::from(image.height())).ceil().max(1.0) as u32;
    let width = width.min(image.width().saturating_sub(x).max(1));
    let height = height.min(image.height().saturating_sub(y).max(1));
    imageops::crop_imm(image, x, y, width, height).to_image()
}

fn enhance_text_image(image: &RgbaImage) -> RgbaImage {
    let mut enhanced = RgbaImage::new(image.width(), image.height());
    for (x, y, pixel) in image.enumerate_pixels() {
        let [r, g, b, a] = pixel.0;
        let gray = (0.2126 * f64::from(r) + 0.7152 * f64::from(g) + 0.0722 * f64::from(b)) / 255.0;
        let adjusted = ((gray - 0.5) * 1.28 + 0.52).clamp(0.0, 1.0);
        let byte = (adjusted * 255.0).round() as u8;
        enhanced.put_pixel(x, y, image::Rgba([byte, byte, byte, a]));
    }
    imageops::unsharpen(&enhanced, 0.8, 4)
}

fn encode_processed_image(image: &RgbaImage, format: &str) -> Result<Vec<u8>, String> {
    let mut encoded = Vec::new();
    if format == "png" {
        PngEncoder::new(&mut encoded)
            .write_image(
                image.as_raw(),
                image.width(),
                image.height(),
                ColorType::Rgba8.into(),
            )
            .map_err(|error| format!("Failed to encode PNG: {error}"))?;
    } else {
        let rgb = DynamicImage::ImageRgba8(image.clone()).to_rgb8();
        JpegEncoder::new_with_quality(&mut encoded, 86)
            .encode(
                rgb.as_raw(),
                rgb.width(),
                rgb.height(),
                ColorType::Rgb8.into(),
            )
            .map_err(|error| format!("Failed to encode JPEG: {error}"))?;
    }
    Ok(encoded)
}

fn base64_encode(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut output = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0];
        let b1 = *chunk.get(1).unwrap_or(&0);
        let b2 = *chunk.get(2).unwrap_or(&0);
        output.push(TABLE[(b0 >> 2) as usize] as char);
        output.push(TABLE[(((b0 & 0b0000_0011) << 4) | (b1 >> 4)) as usize] as char);
        if chunk.len() > 1 {
            output.push(TABLE[(((b1 & 0b0000_1111) << 2) | (b2 >> 6)) as usize] as char);
        } else {
            output.push('=');
        }
        if chunk.len() > 2 {
            output.push(TABLE[(b2 & 0b0011_1111) as usize] as char);
        } else {
            output.push('=');
        }
    }
    output
}

#[cfg(test)]
mod image_preprocess_tests {
    use super::{analyse_image, parse_normalized_crop};
    use image::{DynamicImage, Rgba, RgbaImage};

    #[test]
    fn image_analysis_detects_blank_image() {
        let image =
            DynamicImage::ImageRgba8(RgbaImage::from_pixel(64, 64, Rgba([255, 255, 255, 255])));

        let analysis = analyse_image(&image, 64);

        assert!(analysis.is_likely_blank());
        assert!(analysis.content_box.is_none());
    }

    #[test]
    fn image_analysis_finds_content_box() {
        let mut image = RgbaImage::from_pixel(100, 80, Rgba([255, 255, 255, 255]));
        for y in 20..50 {
            for x in 30..70 {
                image.put_pixel(x, y, Rgba([0, 0, 0, 255]));
            }
        }
        let image = DynamicImage::ImageRgba8(image);

        let analysis = analyse_image(&image, 100);
        let box_rect = analysis.content_box.expect("content box");

        assert!(!analysis.is_likely_blank());
        assert!(box_rect.width > 0.35);
        assert!(box_rect.width < 0.5);
        assert!(box_rect.height > 0.30);
        assert!(box_rect.height < 0.45);
    }

    #[test]
    fn normalized_crop_clamps_to_image_bounds() {
        let crop = parse_normalized_crop(Some(&serde_json::json!({
            "x": 0.8,
            "y": -1.0,
            "width": 0.5,
            "height": 2.0
        })))
        .expect("crop");

        assert_eq!(crop.x, 0.8);
        assert_eq!(crop.y, 0.0);
        assert!((crop.width - 0.2).abs() < f64::EPSILON);
        assert_eq!(crop.height, 1.0);
    }
}
