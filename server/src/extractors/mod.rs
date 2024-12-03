// extractors/mod.rs

mod docx_extractor;
mod jpeg_extractor;
mod pdf_extractor;
mod xlsx_extractor;

pub use docx_extractor::DocxExtractor;
pub use jpeg_extractor::JpegExtractor;
pub use pdf_extractor::PdfExtractor;
pub use xlsx_extractor::XlsxExtractor;

/// Common trait for all document extractors
pub trait DocumentExtractor {
    fn extract(&self, content: &[u8]) -> Result<Vec<String>, Box<dyn std::error::Error>>;
    fn get_supported_extensions(&self) -> Vec<&'static str>;
}

/// Factory for creating document extractors based on file extension
pub struct DocumentExtractorFactory {
    extractors: Vec<Box<dyn DocumentExtractor>>,
}

impl Default for DocumentExtractorFactory {
    fn default() -> Self {
        let mut factory = DocumentExtractorFactory {
            extractors: Vec::new(),
        };
        factory.extractors.push(Box::new(PdfExtractor));
        factory.extractors.push(Box::new(DocxExtractor));
        factory.extractors.push(Box::new(XlsxExtractor));
        factory.extractors.push(Box::new(JpegExtractor::default()));
        factory
    }
}

impl DocumentExtractorFactory {
    pub fn get_extractor(&self, extension: &str) -> Option<&Box<dyn DocumentExtractor>> {
        self.extractors.iter().find(|extractor| extractor.get_supported_extensions().contains(&extension))
    }
}

/// Main function to extract text from any supported document type
pub fn extract_text_from_document(content: &[u8], extension: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let factory = DocumentExtractorFactory::default();

    if let Some(extractor) = factory.get_extractor(extension) {
        extractor.extract(content)
    } else {
        Err(format!("Unsupported file format: {}", extension).into())
    }
}
