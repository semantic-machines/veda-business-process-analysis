// extractors/types.rs

use std::path::PathBuf;

/// Result of content extraction
#[derive(Debug, Clone)]
pub enum ExtractedContent {
    /// Plain text content
    Text(String),
    /// Reference to image file
    ImageFile {
        /// Path to the image file
        path: String,
        name: String,
        /// Image format (e.g., "jpeg", "png")
        format: String,
        /// Image dimensions if available
        dimensions: Option<(u32, u32)>,
    },
}

/// Configuration for extractors
#[derive(Debug, Clone)]
pub struct ExtractorConfig {
    /// Base directory for storing extracted files
    pub output_dir: PathBuf,
    /// Quality setting for image compression (0-100)
    pub image_quality: u8,
    /// Maximum image dimension (larger images will be scaled down)
    pub max_image_dimension: u32,
}

impl Default for ExtractorConfig {
    fn default() -> Self {
        Self {
            output_dir: PathBuf::from("./extracted_content"),
            image_quality: 85,
            max_image_dimension: 2048,
        }
    }
}

/// Common trait for all document extractors
pub trait DocumentExtractor {
    /// Extract content from binary data
    fn extract(&self, content: &[u8], config: &ExtractorConfig) -> Result<Vec<ExtractedContent>, Box<dyn std::error::Error>>;

    /// Get list of supported file extensions
    fn get_supported_extensions(&self) -> Vec<&'static str>;

    /// Count pages in document
    ///
    /// # Arguments
    /// * `content` - Document content as bytes
    ///
    /// # Returns
    /// * `Result<u32, Box<dyn std::error::Error>>` - Number of pages or error
    fn count_pages(&self, _content: &[u8]) -> Result<u32, Box<dyn std::error::Error>> {
        // Default implementation returns 1 page for text-based documents
        Ok(1)
    }
}
