use crate::common::get_individuals_by_type;
use crate::queue_processor::BusinessProcessAnalysisModule;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_json::{Map, Value};
use std::collections::HashMap;
use v_common::onto::datatype::Lang;
use v_common::onto::individual::Individual;
use v_common::v_api::obj::ResultCode;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Property {
    pub mapping: Option<String>,
    #[serde(rename = "type")]
    pub type_name: Option<String>,
    pub is_multiple: Option<bool>,
    pub items: Option<Box<Property>>,
    pub properties: Option<IndexMap<String, Property>>,
    #[serde(flatten)]
    pub additional: Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseSchema {
    #[serde(rename = "type")]
    pub type_name: String,
    pub properties: IndexMap<String, Property>,
    #[serde(rename = "assign_properties")]
    pub assign_properties: Option<HashMap<String, String>>,
    #[serde(flatten)]
    pub additional: Map<String, Value>,
    #[serde(skip)]
    field_order: Vec<String>,
    #[serde(skip)]
    enum_value_mapping: HashMap<String, String>,
}

#[derive(Debug)]
struct PropertyInfo {
    property_type: String,
    range_type: Option<String>,
    is_class: bool,
    is_multiple: bool,
}

pub struct ParseResult {
    pub main_individual: Individual,
    pub related_individuals: Vec<Individual>,
}

impl ResponseSchema {
    pub fn from_json(json: &str) -> Result<Self, Box<dyn std::error::Error>> {
        //info!("Parsing JSON schema: {}", json);
        let mut schema: ResponseSchema = serde_json::from_str(json)?;

        if let Some(additional_props) = schema.additional.get("assign_properties") {
            if let Some(props_obj) = additional_props.as_object() {
                schema.assign_properties = Some(props_obj.iter().filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string()))).collect());
            }
        }

        schema.field_order = schema.properties.keys().cloned().collect();
        //info!("Field order: {:?}", schema.field_order);

        Ok(schema)
    }

    pub fn to_ai_schema(&mut self, module: &mut BusinessProcessAnalysisModule) -> Result<Value, Box<dyn std::error::Error>> {
        //info!("Converting schema to AI format");
        let mut properties = Map::new();
        let mut enum_mapping = HashMap::new();

        info!("Processing {} fields", self.field_order.len());

        for key in &self.field_order {
            if let Some(prop) = self.properties.get(key) {
                properties.insert(key.clone(), convert_property(key, module, prop, &mut enum_mapping)?);
            }
        }

        self.enum_value_mapping = enum_mapping;

        let schema = json!({
            "type": self.type_name,
            "additionalProperties": false,
            "properties": properties,
            "required": self.field_order
        });

        //if let Some(obj) = schema.as_object_mut() {
        //    if let Some(add_props) = &self.assign_properties {
        //        obj.insert("assign_properties".to_string(), json!(add_props));
        //    }
        //}

        //info!("@A1 self.enum_value_mapping)={:?}", self.enum_value_mapping);
        //info!("@A2 self.properties={:?}", self.properties);

        //info!("Generated AI schema: {}", schema.to_string());
        Ok(schema)
    }

    fn get_property_info(module: &mut BusinessProcessAnalysisModule, property: &str) -> Result<PropertyInfo, Box<dyn std::error::Error>> {
        let mut prop_individual = Individual::default();
        if module.backend.storage.get_individual(property, &mut prop_individual) != ResultCode::Ok {
            error!("Property {} not found in ontology", property);
            return Err(format!("Property {} not found in ontology", property).into());
        }
        prop_individual.parse_all();

        let is_class = prop_individual.any_exists("rdf:type", &["owl:Class"]);
        let is_multiple = !prop_individual.any_exists("rdf:type", &["owl:FunctionalProperty"]);

        let range = prop_individual.get_first_literal("rdfs:range");
        let range_type = if is_class {
            None
        } else if let Some(ref range_uri) = range {
            if !range_uri.starts_with("xsd:") {
                Some(range_uri.clone())
            } else {
                None
            }
        } else {
            None
        };

        let property_type = if is_class {
            "owl:Class".to_string()
        } else {
            range.unwrap_or_else(|| "xsd:string".to_string())
        };

        Ok(PropertyInfo {
            property_type,
            range_type,
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
                if let Some(str_val) = value.as_str() {
                    if is_multiple {
                        individual.add_string(property, str_val, Lang::none());
                    } else {
                        individual.set_string(property, str_val, Lang::none());
                    }
                } else {
                    if is_multiple {
                        individual.add_string(property, &value.to_string(), Lang::none());
                    } else {
                        individual.set_string(property, &value.to_string(), Lang::none());
                    }
                }
            },
        }
        Ok(())
    }

    fn map_ai_value_to_individual(
        value: &Value,
        mapping: &Property,
        module: &mut BusinessProcessAnalysisModule,
        related_individuals: &mut Vec<Individual>,
        parent_individual: &mut Individual,
        property: &str,
        enum_value_mapping: &HashMap<String, String>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if mapping.type_name == Some("array".to_string()) && mapping.items.is_some() {
            if let Value::Array(arr) = value {
                let prop_info = if let Some(mapping_uri) = &mapping.mapping {
                    Self::get_property_info(module, mapping_uri)?
                } else {
                    return Err("No mapping URI provided for array items".into());
                };

                for item in arr {
                    let mut related = Individual::default();
                    related.set_id(&format!("d:{}", uuid::Uuid::new_v4()));

                    // Apply assign_properties from items mapping if present
                    if let Some(items) = &mapping.items {
                        if let Some(assign_props) = &items.additional.get("assign_properties") {
                            if let Some(props) = assign_props.as_object() {
                                for (pred, val) in props {
                                    if let Some(val_str) = val.as_str() {
                                        related.set_uri(pred, val_str);
                                    }
                                }
                            }
                        }
                    }

                    if let Some(range_type) = &prop_info.range_type {
                        related.set_uri("rdf:type", range_type);
                    }

                    if let Some(item_mapping) = &mapping.items {
                        Self::map_ai_value_to_individual(item, item_mapping, module, related_individuals, &mut related, property, enum_value_mapping)?;
                    }

                    if let Some(mapping_uri) = &mapping.mapping {
                        parent_individual.add_uri(mapping_uri, related.get_id());
                    }

                    related_individuals.push(related);
                }
                return Ok(());
            }
        }

        if let Some(properties) = &mapping.properties {
            if let Value::Object(obj) = value {
                // First apply assign_properties from current level
                if let Some(assign_props) = mapping.additional.get("assign_properties") {
                    if let Some(props) = assign_props.as_object() {
                        for (pred, val) in props {
                            if let Some(val_str) = val.as_str() {
                                parent_individual.set_uri(pred, val_str);
                            }
                        }
                    }
                }

                for (key, prop_mapping) in properties {
                    // Get property value or skip iteration
                    let prop_value = match obj.get(key) {
                        Some(value) => value,
                        None => continue,
                    };

                    // Handle non-mapped properties
                    let mapping_uri = match &prop_mapping.mapping {
                        None => {
                            Self::map_ai_value_to_individual(prop_value, prop_mapping, module, related_individuals, parent_individual, key, enum_value_mapping)?;
                            continue;
                        },
                        Some(uri) => uri,
                    };

                    // Get property info
                    let prop_info = Self::get_property_info(module, mapping_uri)?;

                    // Handle enum values if range type exists and it's not a basic xsd type
                    if prop_info.range_type.is_some() && !prop_info.property_type.starts_with("xsd:") {
                        if let Some(input_value) = prop_value.as_str() {
                            let enum_key = format!("{}*{}", key, input_value);

                            if let Some(uri) = enum_value_mapping.get(&enum_key) {
                                if prop_info.is_multiple {
                                    parent_individual.add_uri(mapping_uri, uri);
                                } else {
                                    parent_individual.set_uri(mapping_uri, uri);
                                }
                                continue;
                            }
                        }
                    }

                    // Set property value for non-enum cases
                    Self::set_property_value(parent_individual, mapping_uri, prop_value, &prop_info.property_type, prop_info.is_multiple)?;
                }
                return Ok(());
            }
        }

        if let Some(mapping_uri) = &mapping.mapping {
            let prop_info = Self::get_property_info(module, mapping_uri)?;
            let is_multiple = mapping.is_multiple.unwrap_or(prop_info.is_multiple);

            match value {
                Value::Array(arr) => {
                    if prop_info.is_class {
                        for item in arr {
                            if let Value::Object(_) = item {
                                parent_individual.add_string(mapping_uri, &serde_json::to_string_pretty(item)?, Lang::none());
                            }
                        }
                    } else {
                        for item in arr {
                            if let Some(str_value) = item.as_str() {
                                let enum_key = format!("{}*{}", property, str_value);
                                //info!("Looking up array enum key: {} in property_mapping", enum_key);

                                if let Some(uri) = enum_value_mapping.get(&enum_key) {
                                    //info!("Found URI mapping (array): {} -> {}", enum_key, uri);
                                    parent_individual.add_uri(mapping_uri, uri);
                                } else {
                                    info!("No mapping found for array key: {}", enum_key);
                                    Self::set_property_value(parent_individual, mapping_uri, item, &prop_info.property_type, true)?;
                                }
                            }
                        }
                    }
                },
                Value::Object(_) if prop_info.is_class => {
                    if is_multiple {
                        parent_individual.add_string(mapping_uri, &serde_json::to_string_pretty(value)?, Lang::none());
                    } else {
                        parent_individual.set_string(mapping_uri, &serde_json::to_string_pretty(value)?, Lang::none());
                    }
                },
                _ => {
                    if let Some(str_value) = value.as_str() {
                        let enum_key = format!("{}*{}", property, str_value);
                        //info!("Looking up enum key: {} in property_mapping", enum_key);

                        if let Some(uri) = enum_value_mapping.get(&enum_key) {
                            //info!("Found URI mapping: {} -> {}", enum_key, uri);
                            if is_multiple {
                                //info!("Adding multiple URI: {}", uri);
                                parent_individual.add_uri(mapping_uri, uri);
                            } else {
                                //info!("Setting single URI: {}", uri);
                                parent_individual.set_uri(mapping_uri, uri);
                            }
                        } else {
                            //info!("No mapping found for key: {}", enum_key);
                            Self::set_property_value(parent_individual, mapping_uri, value, &prop_info.property_type, is_multiple)?;
                        }
                    } else {
                        Self::set_property_value(parent_individual, mapping_uri, value, &prop_info.property_type, is_multiple)?;
                    }
                },
            }
        }

        Ok(())
    }

    pub fn parse_ai_response(&self, response: &Value, module: &mut BusinessProcessAnalysisModule) -> Result<ParseResult, Box<dyn std::error::Error>> {
        let mut result = ParseResult {
            main_individual: Individual::default(),
            related_individuals: Vec::new(),
        };

        let result_id = format!("d:result_{}", uuid::Uuid::new_v4());
        info!("Created result ID: {}", result_id);
        result.main_individual.set_id(&result_id);
        result.main_individual.set_uri("rdf:type", "v-bpa:GenericProcessingResult");

        if let Some(obj) = response.as_object() {
            for (key, prop_mapping) in &self.properties {
                if let Some(value) = obj.get(key) {
                    //info!("Processing field: {} with value: {:?}", key, value);
                    Self::map_ai_value_to_individual(
                        value,
                        prop_mapping,
                        module,
                        &mut result.related_individuals,
                        &mut result.main_individual,
                        key,
                        &self.enum_value_mapping,
                    )?;
                }
            }
        }

        // TODO это надо переделать, assign_properties должен действовать на каждый вложенные индивид (там где он обьявлен)
        if let Some(assign_props) = &self.assign_properties {
            for (pred, val) in assign_props {
                result.main_individual.set_uri(&pred, &val);
            }
        }

        Ok(result)
    }
}

fn process_enum_values(
    module: &mut BusinessProcessAnalysisModule,
    json_field_name: &str,
    range_type: &str,
    mapping_key_prefix: &str,
    enum_value_mapping: &mut HashMap<String, String>,
) -> Option<Vec<String>> {
    if range_type.starts_with("xsd:") {
        return None;
    }

    match get_individuals_by_type(module, range_type) {
        Ok(mut instances) => {
            let enum_values: Vec<String> = instances
                .iter_mut()
                .filter_map(|instance| {
                    let label = instance.get_first_literal_with_lang("rdfs:label", &[Lang::new_from_i64(1)]);
                    if let Some(label) = &label {
                        let map_key = format!("{}*{}", mapping_key_prefix, label);
                        enum_value_mapping.insert(map_key, instance.get_id().to_string());
                        let map_key = format!("{}*{}", json_field_name, label);
                        enum_value_mapping.insert(map_key, instance.get_id().to_string());
                    }
                    label
                })
                .collect();

            if enum_values.is_empty() {
                None
            } else {
                Some(enum_values)
            }
        },
        Err(_) => None,
    }
}

fn add_assign_properties(obj: &mut Map<String, Value>, additional: &Map<String, Value>) {
    for (key, value) in additional {
        // Skip service fields that shouldn't go to AI schema
        if !["mapping", "assign_properties", "create_new_individuals"].contains(&key.as_str()) {
            obj.insert(key.clone(), value.clone());
        }
    }
}

fn process_property_individual(
    module: &mut BusinessProcessAnalysisModule,
    json_field_name: &str,
    mapping_uri: &str,
    enum_value_mapping: &mut HashMap<String, String>,
) -> Option<(String, Vec<String>)> {
    let mut prop_individual = Individual::default();
    if module.backend.storage.get_individual(mapping_uri, &mut prop_individual) != ResultCode::Ok {
        return None;
    }

    prop_individual.parse_all();
    let range_type = prop_individual.get_first_literal("rdfs:range")?;
    let key_prefix = mapping_uri.split(':').last().unwrap_or(mapping_uri);
    let enum_values = process_enum_values(module, json_field_name, &range_type, key_prefix, enum_value_mapping)?;

    Some((range_type, enum_values))
}

fn convert_property(
    json_field_name: &str,
    module: &mut BusinessProcessAnalysisModule,
    prop: &Property,
    enum_value_mapping: &mut HashMap<String, String>,
) -> Result<Value, Box<dyn std::error::Error>> {
    //info!("@ json_field_name={}, prop={:?}", json_field_name, prop);
    match &prop.items {
        Some(items) => {
            // Process items schema first
            let mut items_schema = convert_property(json_field_name, module, items, enum_value_mapping)?;

            // Add required fields if it's an object
            if let Some(obj) = items_schema.as_object_mut() {
                if let Some(props) = obj.get("properties") {
                    if let Some(props_obj) = props.as_object() {
                        // Make all properties required
                        let required: Vec<String> = props_obj.keys().cloned().collect();
                        obj.insert("required".to_string(), json!(required));
                    }
                }
            }

            // Create array schema
            let mut array_schema = json!({
                "type": "array",
                "items": items_schema
            });

            // Handle create_new_individuals flag, default to false
            let create_new = prop.additional.get("create_new_individuals").and_then(|v| v.as_bool()).unwrap_or(false);

            if !create_new {
                if let Some(mapping_uri) = &prop.mapping {
                    if let Some((_, enum_values)) = process_property_individual(module, json_field_name, mapping_uri, enum_value_mapping) {
                        if let Some(items) = array_schema.get_mut("items") {
                            if let Some(items_obj) = items.as_object_mut() {
                                items_obj.insert("enum".to_string(), json!(enum_values));
                            }
                        }
                    }
                }
            }

            if let Some(obj) = array_schema.as_object_mut() {
                add_assign_properties(obj, &prop.additional);
            }
            Ok(array_schema)
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
                    //info!("Processing nested property: {}", key);
                    let mut prop_schema = convert_property(key, module, value, enum_value_mapping)?;

                    if let Some(mapping_uri) = &value.mapping {
                        if let Some((_, enum_values)) = process_property_individual(module, json_field_name, mapping_uri, enum_value_mapping) {
                            if let Some(obj) = prop_schema.as_object_mut() {
                                obj.insert("enum".to_string(), json!(enum_values));
                            }
                        }
                    }

                    properties.insert(key.clone(), prop_schema);
                }

                obj.insert("properties".to_string(), Value::Object(properties));
                add_assign_properties(obj, &prop.additional);

                if let Some(required) = prop.additional.get("required") {
                    obj.insert("required".to_string(), required.clone());
                }
            }

            Ok(props_json)
        },
        None => {
            let mut prop_json = json!({
                "type": prop.type_name.clone().unwrap_or_else(|| "string".to_string())
            });

            if let Some(mapping_uri) = &prop.mapping {
                if let Some((_, enum_values)) = process_property_individual(module, json_field_name, mapping_uri, enum_value_mapping) {
                    if let Some(obj) = prop_json.as_object_mut() {
                        //info!("@ prop={:?}", prop);
                        obj.insert("enum".to_string(), json!(enum_values));
                    }
                }
            }

            if let Some(obj) = prop_json.as_object_mut() {
                add_assign_properties(obj, &prop.additional);
            }

            Ok(prop_json)
        },
    }
}
