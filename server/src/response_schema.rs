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
        info!("Parsing JSON schema: {}", json);
        let mut schema: ResponseSchema = serde_json::from_str(json)?;

        if let Some(additional_props) = schema.additional.get("additional_properties") {
            if let Some(props_obj) = additional_props.as_object() {
                schema.additional_properties = Some(props_obj.iter().filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string()))).collect());
            }
        }

        schema.field_order = schema.properties.keys().cloned().collect();
        info!("Field order: {:?}", schema.field_order);

        Ok(schema)
    }

    pub fn to_ai_schema(&mut self, module: &mut BusinessProcessAnalysisModule) -> Result<Value, Box<dyn std::error::Error>> {
        info!("Converting schema to AI format");
        let mut properties = Map::new();
        let mut enum_mapping = HashMap::new();

        info!("Processing {} fields", self.field_order.len());

        for key in &self.field_order {
            if let Some(prop) = self.properties.get(key) {
                properties.insert(key.clone(), convert_property(module, prop, &mut enum_mapping)?);
            }
        }

        self.enum_value_mapping = enum_mapping;

        let mut schema = json!({
            "type": self.type_name,
            "additionalProperties": false,
            "properties": properties,
            "required": self.field_order
        });

        if let Some(obj) = schema.as_object_mut() {
            if let Some(add_props) = &self.additional_properties {
                obj.insert("additional_properties".to_string(), json!(add_props));
            }
        }

        info!("Generated AI schema: {}", schema.to_string());
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

                    if let Some(range_type) = &prop_info.range_type {
                        related.set_uri("rdf:type", range_type);
                    }

                    if let Some(item_mapping) = &mapping.items {
                        Self::process_value(item, item_mapping, module, related_individuals, &mut related, property, enum_value_mapping)?;
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
                for (key, prop_mapping) in properties {
                    if let Some(prop_value) = obj.get(key) {
                        if let Some(mapping_uri) = &prop_mapping.mapping {
                            let prop_info = Self::get_property_info(module, mapping_uri)?;
                            if prop_info.range_type.is_some() {
                                if let Some(input_value) = prop_value.as_str() {
                                    let enum_key = format!("{}*{}", key, input_value);
                                    //info!("Looking up enum key: {} in mapping table", enum_key);

                                    if let Some(uri) = enum_value_mapping.get(&enum_key) {
                                        //info!("Found URI mapping: {} -> {}", enum_key, uri);
                                        if prop_info.is_multiple {
                                            parent_individual.add_uri(mapping_uri, uri);
                                        } else {
                                            parent_individual.set_uri(mapping_uri, uri);
                                        }
                                        continue;
                                    } else {
                                        warn!("No mapping found for key: {}", enum_key);
                                    }
                                }
                            }
                            Self::set_property_value(parent_individual, mapping_uri, prop_value, &prop_info.property_type, prop_info.is_multiple)?;
                        } else {
                            Self::process_value(prop_value, prop_mapping, module, related_individuals, parent_individual, key, enum_value_mapping)?;
                        }
                    }
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

        if let Some(add_props) = &self.additional_properties {
            for (predicate, value) in add_props {
                result.main_individual.set_uri(predicate, value);
            }
        }

        if let Some(obj) = response.as_object() {
            for (key, prop_mapping) in &self.properties {
                if let Some(value) = obj.get(key) {
                    //info!("Processing field: {} with value: {:?}", key, value);
                    Self::process_value(value, prop_mapping, module, &mut result.related_individuals, &mut result.main_individual, key, &self.enum_value_mapping)?;
                }
            }
        }

        Ok(result)
    }
}

fn convert_property(
    module: &mut BusinessProcessAnalysisModule,
    prop: &PropertyMapping,
    property_mapping: &mut HashMap<String, String>,
) -> Result<Value, Box<dyn std::error::Error>> {
    match &prop.items {
        Some(items) => {
            let mut array_schema = json!({
                "type": "array",
                "items": convert_property(module, items, property_mapping)?
            });

            if let Some(mapping_uri) = &prop.mapping {
                let mut prop_individual = Individual::default();
                if module.backend.storage.get_individual(mapping_uri, &mut prop_individual) == ResultCode::Ok {
                    prop_individual.parse_all();

                    if let Some(range_type) = prop_individual.get_first_literal("rdfs:range") {
                        if !range_type.starts_with("xsd:") {
                            //info!("Processing non-xsd range type: {}", range_type);
                            if let Ok(mut instances) = get_individuals_by_type(module, &range_type) {
                                let enum_values: Vec<String> = instances
                                    .iter_mut()
                                    .filter_map(|instance| {
                                        let label = instance.get_first_literal_with_lang("rdfs:label", &[Lang::new_from_i64(1)]);
                                        if let Some(label) = &label {
                                            let short_name = mapping_uri.split(':').last().unwrap_or(mapping_uri);
                                            let map_key = format!("{}*{}", short_name, label);
                                            //info!("Creating enum mapping: {} -> {}", map_key, instance.get_id());
                                            property_mapping.insert(map_key, instance.get_id().to_string());
                                        }
                                        label
                                    })
                                    .collect();

                                //info!("Collected enum values: {:?}", enum_values);
                                if !enum_values.is_empty() {
                                    if let Some(items) = array_schema.get_mut("items") {
                                        if let Some(items_obj) = items.as_object_mut() {
                                            items_obj.insert("enum".to_string(), json!(enum_values));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if let Some(obj) = array_schema.as_object_mut() {
                for (key, value) in &prop.additional {
                    if !["mapping", "is_multiple", "additional_properties"].contains(&key.as_str()) {
                        obj.insert(key.clone(), value.clone());
                    }
                }
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
                    let mut prop_schema = convert_property(module, value, property_mapping)?;

                    if let Some(mapping_uri) = &value.mapping {
                        let mut prop_individual = Individual::default();
                        if module.backend.storage.get_individual(mapping_uri, &mut prop_individual) == ResultCode::Ok {
                            prop_individual.parse_all();

                            if let Some(range_type) = prop_individual.get_first_literal("rdfs:range") {
                                if !range_type.starts_with("xsd:") {
                                    //info!("Processing non-xsd range type for nested property: {}", range_type);
                                    if let Ok(mut instances) = get_individuals_by_type(module, &range_type) {
                                        let enum_values: Vec<String> = instances
                                            .iter_mut()
                                            .filter_map(|instance| {
                                                let label = instance.get_first_literal_with_lang("rdfs:label", &[Lang::new_from_i64(1)]);
                                                if let Some(label) = &label {
                                                    let map_key = format!("{}*{}", key, label);
                                                    //info!("Creating enum mapping for nested property: {} -> {}", map_key, instance.get_id());
                                                    property_mapping.insert(map_key, instance.get_id().to_string());
                                                }
                                                label
                                            })
                                            .collect();

                                        //info!("Collected enum values for nested property: {:?}", enum_values);
                                        if !enum_values.is_empty() {
                                            if let Some(obj) = prop_schema.as_object_mut() {
                                                obj.insert("enum".to_string(), json!(enum_values));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    properties.insert(key.clone(), prop_schema);
                }

                obj.insert("properties".to_string(), Value::Object(properties));

                for (key, value) in &prop.additional {
                    if !["mapping", "is_multiple", "additional_properties"].contains(&key.as_str()) {
                        obj.insert(key.clone(), value.clone());
                    }
                }

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
                let mut prop_individual = Individual::default();
                if module.backend.storage.get_individual(mapping_uri, &mut prop_individual) == ResultCode::Ok {
                    prop_individual.parse_all();
                    //info!("Property individual for simple property: {:?}", prop_individual.get_obj().as_json());

                    if let Some(range_type) = prop_individual.get_first_literal("rdfs:range") {
                        if !range_type.starts_with("xsd:") {
                            //info!("Processing non-xsd range type for simple property: {}", range_type);
                            if let Ok(mut instances) = get_individuals_by_type(module, &range_type) {
                                let enum_values: Vec<String> = instances
                                    .iter_mut()
                                    .filter_map(|instance| {
                                        let label = instance.get_first_literal_with_lang("rdfs:label", &[Lang::new_from_i64(1)]);
                                        if let Some(label) = &label {
                                            let short_name = mapping_uri.split(':').last().unwrap_or(mapping_uri);
                                            let map_key = format!("{}*{}", short_name, label);
                                            //info!("Creating enum mapping for simple property: {} -> {}", map_key, instance.get_id());
                                            property_mapping.insert(map_key, instance.get_id().to_string());
                                        }
                                        label
                                    })
                                    .collect();

                                info!("Collected enum values for simple property: {:?}", enum_values);
                                if !enum_values.is_empty() {
                                    if let Some(obj) = prop_json.as_object_mut() {
                                        obj.insert("enum".to_string(), json!(enum_values));
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if let Some(obj) = prop_json.as_object_mut() {
                for (key, value) in &prop.additional {
                    if !["mapping", "is_multiple", "additional_properties"].contains(&key.as_str()) {
                        obj.insert(key.clone(), value.clone());
                    }
                }
            }

            Ok(prop_json)
        },
    }
}
