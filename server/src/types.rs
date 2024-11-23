// types.rs
use serde_json::Value;
use std::collections::HashMap;

pub type PropertyMapping = HashMap<String, String>; // short_name -> full_name
pub type AIResponseValues = HashMap<String, Value>;
pub type PropertySchema = HashMap<String, Value>;

/*
pub static SYSTEM_PREDICATE: &[&str] = &[
    "rdfs:isDefinedBy",
    "v-s:updateCounter",
    "v-s:created",
];
*/
