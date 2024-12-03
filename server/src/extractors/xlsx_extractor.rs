// extractors/xlsx_extractor.rs

use super::DocumentExtractor;
use calamine::{Reader, Xlsx};
use log::info;
use std::io::Cursor;

/// XLSX document extractor implementation
pub struct XlsxExtractor;

impl DocumentExtractor for XlsxExtractor {
    fn extract(&self, content: &[u8]) -> Result<String, Box<dyn std::error::Error>> {
        info!("Starting XLSX text extraction");
        let cursor = Cursor::new(content);
        let mut workbook: Xlsx<_> = calamine::Xlsx::new(cursor)?;

        let mut text = String::new();

        // Process each worksheet
        for sheet_name in workbook.sheet_names().to_owned() {
            info!("Processing sheet: {}", sheet_name);
            if let Some(Ok(range)) = workbook.worksheet_range(&sheet_name) {
                text.push_str(&format!("Sheet: {}\n", sheet_name));

                // Convert each row to text
                for row in range.rows() {
                    let row_text = row
                        .iter()
                        .map(|cell| cell.to_string())
                        .filter(|s| !s.is_empty()) // Skip empty cells
                        .collect::<Vec<String>>()
                        .join("\t");
                    if !row_text.is_empty() {
                        text.push_str(&row_text);
                        text.push('\n');
                    }
                }
                text.push('\n');
            }
        }

        // Remove extra whitespace and normalize line endings
        text = text.lines().map(|line| line.trim()).filter(|line| !line.is_empty()).collect::<Vec<_>>().join("\n");

        info!("XLSX text extraction completed");
        Ok(text)
    }

    fn get_supported_extensions(&self) -> Vec<&'static str> {
        vec!["xlsx", "xls"]
    }
}
