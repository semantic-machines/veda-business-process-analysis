use serde::{Deserialize, Serialize};
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
    pub properties: Option<HashMap<String, PropertyMapping>>,
    #[serde(flatten)]
    pub additional: Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseSchema {
    #[serde(rename = "type")]
    pub type_name: String,
    pub properties: HashMap<String, PropertyMapping>,
    #[serde(rename = "additional_properties")]
    pub additional_properties: Option<HashMap<String, String>>,
    #[serde(flatten)]
    pub additional: Map<String, Value>,
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
    pub fn from_json(json: &str) -> Result<Self, Box<dyn std::error::Error>> {
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
            let mut map = Map::new();

            if prop.items.is_some() {
                map.insert("type".to_string(), Value::String("array".to_string()));
                map.insert("items".to_string(), convert_property(prop.items.as_ref().unwrap()));
            } else if prop.properties.is_some() {
                map.insert("type".to_string(), Value::String("object".to_string()));
                // Add additionalProperties: false for all objects
                map.insert("additionalProperties".to_string(), Value::Bool(false));

                let mut props_map = Map::new();
                for (key, value) in prop.properties.as_ref().unwrap() {
                    props_map.insert(key.clone(), convert_property(value));
                }
                map.insert("properties".to_string(), Value::Object(props_map));
            } else {
                map.insert("type".to_string(), Value::String(prop.type_name.clone().unwrap_or_else(|| "string".to_string())));
            }

            for (key, value) in &prop.additional {
                if !["mapping", "is_multiple", "additional_properties"].contains(&key.as_str()) {
                    map.insert(key.clone(), value.clone());
                }
            }

            Value::Object(map)
        }

        let mut schema_map = Map::new();
        schema_map.insert("type".to_string(), Value::String(self.type_name.clone()));
        // Add additionalProperties: false for root object
        schema_map.insert("additionalProperties".to_string(), Value::Bool(false));

        let mut props_map = Map::new();
        for (key, value) in &self.properties {
            props_map.insert(key.clone(), convert_property(value));
        }
        schema_map.insert("properties".to_string(), Value::Object(props_map));

        for (key, value) in &self.additional {
            if !["mapping", "is_multiple", "additional_properties"].contains(&key.as_str()) {
                schema_map.insert(key.clone(), value.clone());
            }
        }

        Ok(Value::Object(schema_map))
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
