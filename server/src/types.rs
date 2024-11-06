// types.rs

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum JustificationLevel {
    #[serde(rename = "Полностью обоснован")]
    CompletelyJustified,
    #[serde(rename = "Частично обоснован")]
    PartlyJustified,
    #[serde(rename = "Не обоснован")]
    NotJustified,
}

impl JustificationLevel {
    pub fn to_uri(&self) -> &'static str {
        match self {
            JustificationLevel::CompletelyJustified => "v-bpa:CompletelyJustified",
            JustificationLevel::PartlyJustified => "v-bpa:PartlyJustified",
            JustificationLevel::NotJustified => "v-bpa:NotJustified",
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProcessJustification {
    pub level: JustificationLevel,
}

pub type PropertyMapping = HashMap<String, String>; // short_name -> full_name
pub type AIResponseValues = HashMap<String, serde_json::Value>;
