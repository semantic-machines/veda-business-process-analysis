use super::types::{DocumentExtractor, ExtractedContent, ExtractorConfig};
use image::codecs::jpeg::JpegEncoder;
use log::{info, warn};
use pdf2image::{Pages, RenderOptionsBuilder, Scale, PDF};
use std::fs;
use uuid::Uuid;

pub struct PdfExtractor;

impl DocumentExtractor for PdfExtractor {
    fn extract(&self, content: &[u8], config: &ExtractorConfig) -> Result<Vec<ExtractedContent>, Box<dyn std::error::Error>> {
        info!("Starting PDF to images conversion");

        // Create output directory if it doesn't exist
        fs::create_dir_all(&config.output_dir)?;

        let pdf = PDF::from_bytes(content.to_vec())?;
        let render_options = RenderOptionsBuilder::default().scale(Scale::Uniform(config.max_image_dimension)).build()?;

        let all_pages = pdf.render(Pages::All, render_options)?;
        let real_page_count = all_pages.len();

        info!("PDF document has {} pages", real_page_count);

        if real_page_count == 0 {
            return Err("PDF document has no pages".into());
        }

        let mut extracted_contents = Vec::with_capacity(real_page_count);
        let mut last_image_hash = None;

        for (page_num, page) in all_pages.into_iter().enumerate() {
            info!("Processing page {}/{}", page_num + 1, real_page_count);

            let rgb = page.to_rgb8();
            let (width, height) = (rgb.width(), rgb.height());

            if width == 0 || height == 0 {
                warn!("Page {} has invalid dimensions", page_num + 1);
                continue;
            }

            let mut buffer = Vec::new();
            {
                let mut encoder = JpegEncoder::new_with_quality(&mut buffer, config.image_quality);
                encoder.encode(&rgb, width, height, image::ColorType::Rgb8)?;
            }

            // Generate simple hash for duplicate detection
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut hasher = DefaultHasher::new();
            buffer.hash(&mut hasher);
            let image_hash = hasher.finish();

            // Skip duplicate pages
            if let Some(last_hash) = last_image_hash {
                if last_hash == image_hash {
                    warn!("Page {} appears to be identical to previous page, skipping", page_num + 1);
                    continue;
                }
            }
            last_image_hash = Some(image_hash);

            // Create unique filename with page number
            let filename = format!("page_{}_{}.jpg", page_num + 1, Uuid::new_v4());
            let full_file_path = config.output_dir.join(&filename);

            // Save the image
            fs::write(&full_file_path, &buffer)?;
            info!("Saved page {} to {:?}", page_num + 1, &full_file_path);

            extracted_contents.push(ExtractedContent::ImageFile {
                path: config.output_dir.to_str().ok_or("P1284")?.to_string(),
                name: filename,
                format: "jpeg".to_string(),
                dimensions: Some((width, height)),
            });
        }

        info!("Successfully processed {} pages", extracted_contents.len());
        Ok(extracted_contents)
    }

    fn get_supported_extensions(&self) -> Vec<&'static str> {
        vec!["pdf"]
    }
}
