// types.rs
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::HashMap;

pub type PropertyMapping = HashMap<String, String>; // short_name -> full_name
pub type PropertySchema = HashMap<String, Value>;

/*
pub static SYSTEM_PREDICATE: &[&str] = &[
    "rdfs:isDefinedBy",
    "v-s:updateCounter",
    "v-s:created",
];
*/

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIResponseValues {
    pub data: HashMap<String, Value>,
    pub input_tokens: usize,
    pub output_tokens: usize,
}

impl AIResponseValues {
    pub fn new(data: HashMap<String, Value>, input_tokens: usize, output_tokens: usize) -> Self {
        Self {
            data,
            input_tokens,
            output_tokens,
        }
    }

    pub fn get(&self, key: &str) -> Option<&Value> {
        self.data.get(key)
    }

    //pub fn insert(&mut self, key: String, value: Value) {
    //    self.data.insert(key, value);
    //}

    // Helper method to create from single text result
    pub fn from_text(text: String, input_tokens: usize, output_tokens: usize) -> Self {
        let mut data = HashMap::new();
        data.insert("result".to_string(), Value::String(text));
        Self::new(data, input_tokens, output_tokens)
    }

    // Convert internal data to serde_json::Value
    pub fn to_json_value(&self) -> Value {
        let map: Map<String, Value> = self.data.clone().into_iter().collect();
        Value::Object(map)
    }
}
