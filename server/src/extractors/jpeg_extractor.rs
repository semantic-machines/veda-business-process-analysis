use super::DocumentExtractor;
use image::{imageops::FilterType, GenericImageView, ImageFormat, imageops};
use log::{info, error};
use serde_json::json;
use std::io::Cursor;
use std::fs;
use std::path::Path;
use base64::engine::general_purpose::STANDARD;
use base64::Engine;

pub struct JpegExtractor {
    max_size: u64,
    max_dimension: u32,
    jpeg_quality: u8,
    output_dir: String,
}

impl Default for JpegExtractor {
    fn default() -> Self {
        JpegExtractor {
            max_size: 20 * 2048 * 2048,
            max_dimension: 2048,
            jpeg_quality: 80,
            output_dir: String::from("./processed_images"), // Default output directory
        }
    }
}

impl DocumentExtractor for JpegExtractor {
    fn extract(&self, content: &[u8]) -> Result<String, Box<dyn std::error::Error>> {
        info!("Processing JPEG image");

        if content.len() as u64 > self.max_size {
            return Err("Image size exceeds 20MB limit".into());
        }

        let (scaled_content, filename) = self.process_image(content)?;
        let base64_content = STANDARD.encode(&scaled_content);

        // Save processed image
        self.save_processed_image(&filename, &scaled_content)?;

        info!("Base64 encoded size: {} bytes", base64_content.len());

        let response = json!({
            "role": "user",
            "content": [
                {
                    "type": "image_url",
                    "image_url": {
                        "url": format!("data:image/jpeg;base64,{}", base64_content),
                        "detail": "high"
                    }
                }
            ]
        });

        info!("JPEG processing completed successfully");
        Ok(serde_json::to_string(&response)?)
    }

    fn get_supported_extensions(&self) -> Vec<&'static str> {
        vec!["jpg", "jpeg"]
    }
}

impl JpegExtractor {
    pub fn with_output_dir(output_dir: String) -> Self {
        Self {
            output_dir,
            ..Default::default()
        }
    }

    fn process_image(&self, content: &[u8]) -> Result<(Vec<u8>, String), Box<dyn std::error::Error>> {
        let img = image::load_from_memory(content)?;
        let (width, height) = img.dimensions();

        info!("Original image dimensions: {}x{}", width, height);

        // Create unique filename based on timestamp
        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
        let filename = format!("processed_{}.jpg", timestamp);

        // Calculate scaling factor to fit within max dimension
        let scale = self.max_dimension as f32 / width.max(height) as f32;
        let new_width = (width as f32 * scale) as u32;
        let new_height = (height as f32 * scale) as u32;

        info!("Scaling to dimensions: {}x{}", new_width, new_height);

        // Resize image
        let scaled_img = img.resize(new_width, new_height, FilterType::Lanczos3);

        // Convert to grayscale
        let gray_img = imageops::grayscale(&scaled_img);

        // Convert to JPEG with compression
        let mut buffer = Vec::new();
        let mut cursor = Cursor::new(&mut buffer);

        let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut cursor, self.jpeg_quality);
        encoder.encode(
            gray_img.as_raw(),
            gray_img.width(),
            gray_img.height(),
            image::ColorType::L8
        )?;

        info!("Final image size: {} bytes", buffer.len());
        Ok((buffer, filename))
    }

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
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;

    #[test]
    fn test_save_processed_image() {
        let test_dir = "./test_processed_images";
        let extractor = JpegExtractor::with_output_dir(test_dir.to_string());

        // Create simple test image
        let test_content = vec![0u8; 100];
        let test_filename = "test.jpg";

        // Test saving
        assert!(extractor.save_processed_image(test_filename, &test_content).is_ok());

        // Verify file exists and content
        let filepath = Path::new(test_dir).join(test_filename);
        assert!(filepath.exists());
        assert_eq!(fs::read(&filepath).unwrap(), test_content);

        // Cleanup
        fs::remove_dir_all(test_dir).unwrap();
    }
}