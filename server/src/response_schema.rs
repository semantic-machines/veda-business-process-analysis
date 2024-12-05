use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_json::{Map, Value};
use std::collections::HashMap;
use v_common::module::veda_backend::Backend;
use v_common::onto::datatype::Lang;
use v_common::onto::individual::Individual;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertyMapping {
    pub mapping: Option<String>,
    #[serde(rename = "type")]
    pub type_name: Option<String>,
    pub is_multiple: Option<bool>,
    pub items: Option<Box<PropertyMapping>>,
    pub properties: Option<IndexMap<String, PropertyMapping>>,
    #[serde(flatten)]
    pub additional: Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseSchema {
    #[serde(rename = "type")]
    pub type_name: String,
    pub properties: IndexMap<String, PropertyMapping>,
    #[serde(rename = "additional_properties")]
    pub additional_properties: Option<HashMap<String, String>>,
    #[serde(flatten)]
    pub additional: Map<String, Value>,

    #[serde(skip)]
    field_order: Vec<String>, // Store original field order
}

#[derive(Debug)]
struct PropertyInfo {
    property_type: String,
    is_class: bool,
    is_multiple: bool,
}

pub struct ParseResult {
    pub main_individual: Individual,
    pub related_individuals: Vec<Individual>,
}

impl ResponseSchema {
    pub fn from_json0(json: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let mut schema: ResponseSchema = serde_json::from_str(json)?;
        if schema.type_name != "object" {
            return Err("Root schema type must be 'object'".into());
        }

        // Проверяем наличие additional_properties в дополнительных полях
        if let Some(additional_props) = schema.additional.get("additional_properties") {
            if let Some(props_obj) = additional_props.as_object() {
                schema.additional_properties = Some(props_obj.iter().filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string()))).collect());
            }
        }

        Ok(schema)
    }
    pub fn from_json(json: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let mut schema: ResponseSchema = serde_json::from_str(json)?;

        // Проверяем наличие additional_properties в дополнительных полях
        if let Some(additional_props) = schema.additional.get("additional_properties") {
            if let Some(props_obj) = additional_props.as_object() {
                schema.additional_properties = Some(props_obj.iter().filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string()))).collect());
            }
        }

        // Extract field order from original JSON
        schema.field_order = schema.properties.keys().cloned().collect();

        Ok(schema)
    }

    pub fn from_value(value: &Value) -> Result<Self, Box<dyn std::error::Error>> {
        let mut schema: ResponseSchema = serde_json::from_value(value.clone())?;
        if schema.type_name != "object" {
            return Err("Root schema type must be 'object'".into());
        }

        // Проверяем наличие additional_properties в дополнительных полях
        if let Some(additional_props) = value.get("additional_properties") {
            if let Some(props_obj) = additional_props.as_object() {
                schema.additional_properties = Some(props_obj.iter().filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string()))).collect());
            }
        }

        Ok(schema)
    }

    pub fn to_ai_schema(&self) -> Result<Value, Box<dyn std::error::Error>> {
        fn convert_property(prop: &PropertyMapping) -> Value {
            match &prop.items {
                Some(items) => {
                    let mut array_schema = json!({
                        "type": "array",
                        "items": convert_property(items)
                    });

                    // Copy any additional fields from the parent property
                    if let Some(obj) = array_schema.as_object_mut() {
                        for (key, value) in &prop.additional {
                            if !["mapping", "is_multiple", "additional_properties"].contains(&key.as_str()) {
                                obj.insert(key.clone(), value.clone());
                            }
                        }
                    }
                    array_schema
                },
                None if prop.properties.is_some() => {
                    let props = prop.properties.as_ref().unwrap();
                    let mut props_json = json!({
                        "type": "object",
                        "additionalProperties": false,
                    });

                    if let Some(obj) = props_json.as_object_mut() {
                        let mut properties = Map::new();
                        for (key, value) in props {
                            properties.insert(key.clone(), convert_property(value));
                        }
                        obj.insert("properties".to_string(), Value::Object(properties));

                        // Add any additional fields from the property definition
                        for (key, value) in &prop.additional {
                            if !["mapping", "is_multiple", "additional_properties"].contains(&key.as_str()) {
                                obj.insert(key.clone(), value.clone());
                            }
                        }

                        // Add required fields if specified in the properties
                        if let Some(required) = prop.additional.get("required") {
                            obj.insert("required".to_string(), required.clone());
                        }
                    }

                    props_json
                },
                None => {
                    let mut prop_json = json!({
                        "type": prop.type_name.clone().unwrap_or_else(|| "string".to_string())
                    });

                    // Copy additional fields like enum, description etc.
                    if let Some(obj) = prop_json.as_object_mut() {
                        for (key, value) in &prop.additional {
                            if !["mapping", "is_multiple", "additional_properties"].contains(&key.as_str()) {
                                obj.insert(key.clone(), value.clone());
                            }
                        }
                    }

                    prop_json
                },
            }
        }

        // Build properties json maintaining field order
        let mut properties = Map::new();
        for key in &self.field_order {
            if let Some(prop) = self.properties.get(key) {
                properties.insert(key.clone(), convert_property(prop));
            }
        }

        // Build final schema with additional properties if specified
        let mut schema = json!({
            "type": self.type_name,
            "additionalProperties": false,
            "properties": properties,
            "required": self.field_order
        });

        // Add any additional root level properties
        if let Some(obj) = schema.as_object_mut() {
            if let Some(add_props) = &self.additional_properties {
                obj.insert("additional_properties".to_string(), json!(add_props));
            }
        }

        Ok(schema)
    }

    fn get_property_info(storage: &mut Backend, property: &str) -> Result<PropertyInfo, Box<dyn std::error::Error>> {
        let mut prop_individual = Individual::default();

        if storage.get_individual(property, &mut prop_individual).is_none() {
            return Err(format!("Property {} not found in ontology", property).into());
        }

        let is_class = prop_individual.any_exists("rdf:type", &["owl:Class"]);
        let is_multiple = !prop_individual.any_exists("rdf:type", &["owl:FunctionalProperty"]);
        let property_type = if is_class {
            "owl:Class".to_string()
        } else {
            prop_individual.get_first_literal("rdfs:range").unwrap_or_else(|| "xsd:string".to_string())
        };

        Ok(PropertyInfo {
            property_type,
            is_class,
            is_multiple,
        })
    }

    fn set_property_value(individual: &mut Individual, property: &str, value: &Value, property_type: &str, is_multiple: bool) -> Result<(), Box<dyn std::error::Error>> {
        match property_type {
            "xsd:string" | "xsd:dateTime" => {
                if let Some(s) = value.as_str() {
                    if is_multiple {
                        individual.add_string(property, s, Lang::none());
                    } else {
                        individual.set_string(property, s, Lang::none());
                    }
                }
            },
            "xsd:integer" => {
                if let Some(n) = value.as_i64() {
                    if is_multiple {
                        individual.add_integer(property, n);
                    } else {
                        individual.set_integer(property, n);
                    }
                }
            },
            "xsd:decimal" => {
                if let Some(n) = value.as_f64() {
                    if is_multiple {
                        individual.add_decimal_from_f64(property, n);
                    } else {
                        individual.set_decimal_from_f64(property, n);
                    }
                }
            },
            _ => {
                if is_multiple {
                    individual.add_string(property, &value.to_string(), Lang::none());
                } else {
                    individual.set_string(property, &value.to_string(), Lang::none());
                }
            },
        }
        Ok(())
    }

    fn process_value(
        value: &Value,
        mapping: &PropertyMapping,
        storage: &mut Backend,
        related_individuals: &mut Vec<Individual>,
        parent_individual: &mut Individual,
        property: &str,
        sys_ticket: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        println!("\nProcessing property: {}", property);
        println!("Value: {:#?}", value);
        println!("Mapping: {:#?}", mapping);

        // Handle arrays with mapping
        if mapping.type_name == Some("array".to_string()) && mapping.mapping.is_some() {
            let mapping_uri = mapping.mapping.as_ref().unwrap();
            match value {
                Value::Array(arr) => {
                    // Convert array to JSON string if it contains objects
                    if arr.iter().any(|v| v.is_object()) {
                        let json_str = serde_json::to_string_pretty(arr)?;
                        parent_individual.add_string(mapping_uri, &json_str, Lang::none());
                    } else {
                        // Handle simple array values
                        for item in arr {
                            if let Some(str_val) = item.as_str() {
                                parent_individual.add_string(mapping_uri, str_val, Lang::none());
                            }
                        }
                    }
                    return Ok(());
                },
                _ => return Ok(()),
            }
        }

        // Rest of the existing code...
        if let Some(properties) = &mapping.properties {
            println!("Found properties for {}", property);
            if let Value::Object(obj) = value {
                for (key, prop_mapping) in properties {
                    println!("\nHandling nested property: {}", key);
                    if let Some(prop_value) = obj.get(key) {
                        if let Some(mapping_uri) = &prop_mapping.mapping {
                            println!("Found mapping URI for {}: {}", key, mapping_uri);
                            let property_info = Self::get_property_info(storage, mapping_uri)?;

                            Self::set_property_value(parent_individual, mapping_uri, prop_value, &property_info.property_type, property_info.is_multiple)?;
                        } else {
                            Self::process_value(prop_value, prop_mapping, storage, related_individuals, parent_individual, key, sys_ticket)?;
                        }
                    }
                }
                return Ok(());
            }
        }

        // Direct mapping handling
        if let Some(mapping_uri) = &mapping.mapping {
            println!("\nHandling direct mapping for: {}", mapping_uri);
            let property_info = Self::get_property_info(storage, mapping_uri)?;
            let is_multiple = mapping.is_multiple.unwrap_or(property_info.is_multiple);

            match value {
                Value::Array(arr) => {
                    if property_info.is_class {
                        for item in arr {
                            // Save as JSON for unmapped properties
                            if let Value::Object(_) = item {
                                parent_individual.add_string(mapping_uri, &serde_json::to_string_pretty(item)?, Lang::none());
                            }
                        }
                    } else {
                        for item in arr {
                            Self::set_property_value(parent_individual, mapping_uri, item, &property_info.property_type, true)?;
                        }
                    }
                },
                Value::Object(_obj) if property_info.is_class => {
                    // Save as JSON for object without mapping
                    if is_multiple {
                        parent_individual.add_string(mapping_uri, &serde_json::to_string_pretty(value)?, Lang::none());
                    } else {
                        parent_individual.set_string(mapping_uri, &serde_json::to_string_pretty(value)?, Lang::none());
                    }
                },
                _ => {
                    Self::set_property_value(parent_individual, mapping_uri, value, &property_info.property_type, is_multiple)?;
                },
            }
        }
        Ok(())
    }

    pub fn parse_ai_response(&self, response: &Value, storage: &mut Backend, sys_ticket: &str) -> Result<ParseResult, Box<dyn std::error::Error>> {
        let mut result = ParseResult {
            main_individual: Individual::default(),
            related_individuals: Vec::new(),
        };

        result.main_individual.set_id(&format!("d:result_{}", uuid::Uuid::new_v4()));

        // Применяем additional_properties если они есть
        if let Some(additional_props) = &self.additional_properties {
            println!("Applying additional properties: {:?}", additional_props);
            for (predicate, value) in additional_props {
                println!("Setting additional property {} = {}", predicate, value);
                result.main_individual.set_uri(predicate, value);
            }
        }

        if let Some(obj) = response.as_object() {
            for (key, prop_mapping) in &self.properties {
                if let Some(value) = obj.get(key) {
                    Self::process_value(value, prop_mapping, storage, &mut result.related_individuals, &mut result.main_individual, key, sys_ticket)?;
                }
            }
        }

        Ok(result)
    }
}
