use super::DocumentExtractor;
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use image::codecs::jpeg::JpegEncoder;
use log::info;
use pdf2image::{RenderOptionsBuilder, Scale, PDF};
use std::fs;
use std::path::Path;

pub struct PdfExtractor;

impl DocumentExtractor for PdfExtractor {
    fn extract(&self, content: &[u8]) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        info!("Starting PDF to images conversion");

        let pdf = PDF::from_bytes(content.to_vec())?;
        let render_options = RenderOptionsBuilder::default()
            .scale(Scale::Uniform(2048)) // Will maintain aspect ratio and fit within 2048x2048
            .build()?;

        let pages = pdf.render(pdf2image::Pages::All, render_options)?;
        info!("Converted {} PDF pages to images", pages.len());

        let debug_dir = Path::new("debug_images");
        if !debug_dir.exists() {
            fs::create_dir(debug_dir)?;
        }

        let base64_pages = pages
            .into_iter()
            .enumerate()
            .map(|(idx, page)| {
                let mut buffer = Vec::new();
                {
                    let mut encoder = JpegEncoder::new_with_quality(&mut buffer, 100);
                    let rgb = page.to_rgb8();
                    encoder.encode(&rgb, rgb.width(), rgb.height(), image::ColorType::Rgb8)?;
                }

                // Save for debug
                fs::write(debug_dir.join(format!("page_{}.jpg", idx + 1)), &buffer)?;

                Ok(STANDARD.encode(&buffer))
            })
            .collect::<Result<Vec<String>, Box<dyn std::error::Error>>>()?;

        Ok(base64_pages)
    }

    fn get_supported_extensions(&self) -> Vec<&'static str> {
        vec!["pdf"]
    }
}
