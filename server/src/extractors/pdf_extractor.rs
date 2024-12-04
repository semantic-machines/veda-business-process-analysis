use super::DocumentExtractor;
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use image::codecs::jpeg::JpegEncoder;
use log::{info, warn};
use pdf2image::{Pages, RenderOptionsBuilder, Scale, PDF};
use std::fs;
use std::path::Path;

pub struct PdfExtractor;

impl DocumentExtractor for PdfExtractor {
    fn extract(&self, content: &[u8]) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        info!("Starting PDF to images conversion");

        let pdf = PDF::from_bytes(content.to_vec())?;

        let render_options = RenderOptionsBuilder::default().scale(Scale::Uniform(2048)).build()?;

        let all_pages = pdf.render(Pages::All, render_options)?;
        let real_page_count = all_pages.len();
        info!("PDF document has {} pages (reported by page_count) and {} actual pages", pdf.page_count(), real_page_count);

        if real_page_count == 0 {
            return Err("PDF document has no pages".into());
        }

        let mut base64_pages = Vec::with_capacity(real_page_count);
        let debug_dir = Path::new("debug_images");
        if !debug_dir.exists() {
            fs::create_dir(debug_dir)?;
        }

        let mut last_page_hash = None;

        for (page_num, page) in all_pages.into_iter().enumerate() {
            info!("Processing page {}/{}", page_num + 1, real_page_count);

            let mut buffer = Vec::new();

            let rgb = page.to_rgb8();
            let (width, height) = (rgb.width(), rgb.height());
            info!("Page {} dimensions: {}x{}", page_num + 1, width, height);

            if width == 0 || height == 0 {
                warn!("Page {} has invalid dimensions", page_num + 1);
                continue;
            }

            let pixels_sum: u32 = rgb.pixels().map(|p| p[0] as u32 + p[1] as u32 + p[2] as u32).sum();
            let avg_brightness = pixels_sum as f32 / (width as f32 * height as f32 * 3.0);
            info!("Page {} average brightness: {:.2}", page_num + 1, avg_brightness);

            {
                let mut encoder = JpegEncoder::new_with_quality(&mut buffer, 100);
                encoder.encode(&rgb, width, height, image::ColorType::Rgb8)?;
            }

            let buffer_size = buffer.len();
            info!("Page {} encoded JPEG size: {} bytes", page_num + 1, buffer_size);

            // Calculate page hash
            let current_page_hash = STANDARD.encode(&buffer);

            // Check for duplicate page
            if let Some(last_hash) = &last_page_hash {
                if *last_hash == current_page_hash {
                    warn!("Page {} appears to be identical to previous page, skipping", page_num + 1);
                    continue;
                }
            }

            // Save debug image and update hash only for non-duplicate pages
            let debug_path = debug_dir.join(format!("page_{}.jpg", page_num + 1));
            fs::write(&debug_path, &buffer)?;
            info!("Saved debug image for page {} to {:?}", page_num + 1, debug_path);

            info!("Page {} base64 length: {}", page_num + 1, current_page_hash.len());

            base64_pages.push(current_page_hash.clone());
            last_page_hash = Some(current_page_hash);
        }

        info!("Successfully processed {} pages", base64_pages.len());

        Ok(base64_pages)
    }

    fn get_supported_extensions(&self) -> Vec<&'static str> {
        vec!["pdf"]
    }
}
