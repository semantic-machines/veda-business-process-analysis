// extractors/jpeg_extractor.rs

use super::DocumentExtractor;
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use image::{imageops, imageops::FilterType, GenericImageView};
use log::info;
use std::fs;
use std::io::Cursor;
use std::path::Path;

/// JPEG image extractor with support for resizing and quality parameters
pub struct JpegExtractor {
    max_size: u64,
    max_dimension: u32,
    jpeg_quality: u8,
    output_dir: String,
}

impl JpegExtractor {
    fn save_processed_image(&self, filename: &str, content: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        // Create output directory if it doesn't exist
        if !Path::new(&self.output_dir).exists() {
            fs::create_dir_all(&self.output_dir)?;
        }

        let filepath = Path::new(&self.output_dir).join(filename);
        info!("Saving processed image to: {}", filepath.display());

        match fs::write(&filepath, content) {
            Ok(_) => {
                info!("Successfully saved processed image");
                Ok(())
            },
            Err(e) => {
                error!("Failed to save processed image: {}", e);
                Err(Box::new(e))
            },
        }
    }
}

impl Default for JpegExtractor {
    fn default() -> Self {
        JpegExtractor {
            max_size: 20 * 1024 * 1024,
            max_dimension: 2048,
            jpeg_quality: 100,
            output_dir: String::from("./processed_images"),
        }
    }
}

impl DocumentExtractor for JpegExtractor {
    fn extract(&self, content: &[u8]) -> Result<String, Box<dyn std::error::Error>> {
        info!("Processing JPEG image");

        if content.len() as u64 > self.max_size {
            return Err("Image size exceeds 20MB limit".into());
        }

        let img = image::load_from_memory(content)?;
        let (width, height) = img.dimensions();
        info!("Original image dimensions: {}x{}", width, height);

        // Scale image according to max dimension
        let scale = self.max_dimension as f32 / width.max(height) as f32;
        let new_width = (width as f32 * scale) as u32;
        let new_height = (height as f32 * scale) as u32;

        info!("Scaling to dimensions: {}x{}", new_width, new_height);

        // Resize image
        let scaled_img = img.resize(new_width, new_height, FilterType::Lanczos3);

        // Convert to grayscale to reduce size
        let gray_img = imageops::grayscale(&scaled_img);

        // Convert to JPEG with compression
        let mut buffer = Vec::new();
        let mut cursor = Cursor::new(&mut buffer);

        let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut cursor, self.jpeg_quality);
        encoder.encode(gray_img.as_raw(), gray_img.width(), gray_img.height(), image::ColorType::L8)?;

        info!("Final image size: {} bytes", buffer.len());
        self.save_processed_image("out.jpg", &buffer)?;

        // Convert to base64
        let base64_content = STANDARD.encode(&buffer);
        info!("Base64 encoded size: {} bytes", base64_content.len());

        Ok(base64_content)
    }

    fn get_supported_extensions(&self) -> Vec<&'static str> {
        vec!["jpg", "jpeg"]
    }
}
