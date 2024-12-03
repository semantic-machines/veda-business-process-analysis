// extractors/pdf_extractor.rs

use super::DocumentExtractor;
use log::info;

/// PDF document extractor implementation
pub struct PdfExtractor;

impl DocumentExtractor for PdfExtractor {
    fn extract(&self, content: &[u8]) -> Result<String, Box<dyn std::error::Error>> {
        info!("Extracting text from PDF document");
        let text = pdf_extract::extract_text_from_mem(content)?;
        Ok(text)
    }

    fn get_supported_extensions(&self) -> Vec<&'static str> {
        vec!["pdf"]
    }
}
