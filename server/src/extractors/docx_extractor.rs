use super::types::{DocumentExtractor, ExtractedContent, ExtractorConfig};
use docx_rs::{DocumentChild, ParagraphChild, RunChild};
use log::info;

/// DOCX document extractor implementation
pub struct DocxExtractor;

impl DocumentExtractor for DocxExtractor {
    fn extract(&self, content: &[u8], _config: &ExtractorConfig) -> Result<Vec<ExtractedContent>, Box<dyn std::error::Error>> {
        info!("Starting DOCX text extraction");
        let docx = docx_rs::read_docx(content)?;

        let mut text = String::new();

        // Process each paragraph and table in the document
        for child in docx.document.children {
            match child {
                DocumentChild::Paragraph(paragraph) => {
                    // Extract text from paragraph
                    let mut para_text = String::new();
                    for child in paragraph.children {
                        match child {
                            ParagraphChild::Run(run) => {
                                for child in run.children {
                                    if let RunChild::Text(text_element) = child {
                                        para_text.push_str(&text_element.text);
                                    }
                                }
                            },
                            _ => {},
                        }
                    }
                    // Add paragraph text only if not empty
                    if !para_text.trim().is_empty() {
                        text.push_str(&para_text);
                        text.push('\n');
                    }
                },
                DocumentChild::Table(table) => {
                    // Process table content
                    text.push_str("\n[TABLE]\n");

                    // Get number of columns from grid
                    let cols = table.grid.len();
                    text.push_str(&format!("Columns: {}\n", cols));

                    text.push_str("[/TABLE]\n");
                },
                _ => {},
            }
        }

        // Remove extra whitespace and normalize line endings
        text = text.lines().map(|line| line.trim()).filter(|line| !line.is_empty()).collect::<Vec<_>>().join("\n");

        info!("DOCX text extraction completed");
        Ok(vec![ExtractedContent::Text(text)])
    }

    fn get_supported_extensions(&self) -> Vec<&'static str> {
        vec!["docx", "doc"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_docx_extractor_extensions() {
        let extractor = DocxExtractor;
        assert_eq!(extractor.get_supported_extensions(), vec!["docx", "doc"]);
    }

    #[test]
    fn test_empty_document() {
        let extractor = DocxExtractor;
        let config = ExtractorConfig {
            output_dir: PathBuf::from("test_output"),
            image_quality: 85,
            max_image_dimension: 2048,
        };

        let result = extractor.extract(&[], &config);
        assert!(result.is_err());
    }
}
