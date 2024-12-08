// common.rs
use crate::queue_processor::BusinessProcessAnalysisModule;
use crate::types::{AIResponseValues, PropertyMapping, PropertySchema};
use humantime::format_duration;
use openai_dive::v1::resources::chat::{
    ChatCompletionParameters, ChatCompletionParametersBuilder, ChatCompletionResponseFormat, ChatMessage, ChatMessageContent, JsonSchemaBuilder,
};
use serde_json::Value;
use std::collections::HashSet;
use std::time::Duration;
use std::{io, thread, time};
use v_common::onto::datatype::Lang;
use v_common::onto::individual::Individual;
use v_common::search::common::{FTQuery, QueryResult};
use v_common::v_api::obj::ResultCode;

#[derive(Debug, Clone, Copy)]
pub enum ClientType {
    Default,
    Reasoning,
}

/// Gets prompt text from ontology individual
///
/// # Arguments
/// * `module` - Business process analysis module
/// * `prompt_id` - ID of prompt individual
///
/// # Returns
/// * `Result<String, Box<dyn std::error::Error>>` - Prompt text or error
pub fn get_prompt_text(module: &mut BusinessProcessAnalysisModule, prompt_id: &str) -> Result<String, Box<dyn std::error::Error>> {
    // Load prompt individual
    let mut prompt_individual = Individual::default();
    if module.backend.storage.get_individual(prompt_id, &mut prompt_individual) != ResultCode::Ok {
        return Err(format!("Failed to get prompt with ID: {}", prompt_id).into());
    }

    prompt_individual.parse_all();

    // Get prompt text
    let prompt_text = prompt_individual.get_first_literal("v-bpa:promptText").ok_or("Prompt text not found")?;

    Ok(prompt_text)
}
/// Формирует JSON-представление бизнес-процесса из индивида, включая связанные документы
///
/// # Arguments
/// * `bp_obj` - Индивид бизнес-процесса
/// * `module` - Модуль с доступом к хранилищу
///
/// # Returns
/// * `Result<Value, Box<dyn std::error::Error>>` - JSON-представление бизнес-процесса
pub fn extract_process_json(bp_obj: &mut Individual, module: &mut BusinessProcessAnalysisModule) -> Result<Value, Box<dyn std::error::Error>> {
    let process_name = bp_obj
        .get_first_literal("rdfs:label")
        .or_else(|| bp_obj.get_first_literal("v-bpa:processName"))
        .ok_or_else(|| format!("Отсутствует название процесса, src={}", bp_obj.get_obj().as_json()))?;

    let process_description = bp_obj.get_first_literal("v-bpa:processDescription").unwrap_or_default();
    let process_participants = bp_obj.get_first_literal("v-bpa:processParticipant").unwrap_or_default();
    let responsible_department = bp_obj.get_first_literal("v-bpa:responsibleDepartment").unwrap_or_default();
    let process_frequency = bp_obj.get_first_literal("v-bpa:processFrequency").unwrap_or_default();
    let labor_costs = bp_obj.get_first_literal("v-bpa:laborCosts").unwrap_or_default();

    // Собираем документы
    let mut documents = Vec::new();
    let document_refs = bp_obj.get_literals_nm("v-bpa:hasProcessDocument").unwrap_or_default();
    for ref_id in document_refs {
        let mut document = Individual::default();
        if module.backend.storage.get_individual(&ref_id, &mut document) == ResultCode::Ok {
            document.parse_all();
            let document_json = serde_json::json!({
                "name": document.get_first_literal("v-bpa:documentName").unwrap_or_default(),
                "content": document.get_first_literal("v-bpa:documentContent").unwrap_or_default()
            });
            documents.push(document_json);
        } else {
            error!("Не удалось загрузить документ обоснования с ID: {}", ref_id);
        }
    }

    let json_value = serde_json::json!({
        "processName": process_name,
        "processDescription": process_description,
        "participants": process_participants,
        "responsibleDepartment": responsible_department,
        "frequency": process_frequency,
        "laborCosts": labor_costs,
        "hasProcessDocument": documents
    });

    Ok(json_value)
}

pub fn get_individuals_by_type(module: &mut BusinessProcessAnalysisModule, type_uri: &str) -> Result<Vec<Individual>, Box<dyn std::error::Error>> {
    let res = get_individuals_uris_by_type(module, type_uri)?;

    let mut individuals = Vec::new();

    // Загружаем каждый найденный индивид
    for id in res {
        let mut individual = Individual::default();
        if module.backend.storage.get_individual(&id, &mut individual) == ResultCode::Ok {
            individual.parse_all();
            individuals.push(individual);
        } else {
            warn!("Failed to load individual {}", id);
        }
    }
    info!("Successfully found and loaded {} individuals of type {}", individuals.len(), type_uri);

    Ok(individuals)
}

/// Находит все индивиды заданного типа в системе
pub fn get_individuals_uris_by_type(module: &mut BusinessProcessAnalysisModule, type_uri: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let query = format!("'rdf:type' === '{}'", type_uri);
    get_individuals_uris_by_query(module, &query)
}

pub fn get_individuals_uris_by_query(module: &mut BusinessProcessAnalysisModule, query: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    //info!("Starting search for individuals of query: {}", query);

    let mut res = QueryResult::default();
    res.result_code = ResultCode::NotReady;

    let mut retry_count = 0;
    const MAX_RETRIES: u32 = 3;

    // Формируем запрос для поиска индивидов заданного типа
    while res.result_code == ResultCode::NotReady || res.result_code == ResultCode::DatabaseModifiedError {
        let ft_query = FTQuery::new_with_user("cfg:VedaSystem", &query);

        info!("Attempting to query individuals of query {} (attempt {})", query, retry_count + 1);

        res = module.xr.query(ft_query, &mut module.backend.storage);

        if res.result_code == ResultCode::InternalServerError {
            error!("Search failed with internal server error");
            return Err(io::Error::new(io::ErrorKind::Other, format!("Search failed with error: {:?}", res.result_code)).into());
        }

        if res.result_code != ResultCode::Ok {
            if retry_count >= MAX_RETRIES {
                error!("Max retries reached while searching for query {}", query);
                return Err(io::Error::new(io::ErrorKind::Other, format!("Failed to search after {} attempts", MAX_RETRIES)).into());
            }
            warn!("Failed to search individuals, retry in 3 seconds... (attempt {})", retry_count + 1);
            thread::sleep(time::Duration::from_secs(3));
        } else {
            return Ok(res.result);
        }
        retry_count += 1;
    }

    Ok(res.result)
}

pub fn load_schema(
    module: &mut BusinessProcessAnalysisModule,
    system_prompt_id: &str,
    excluded: Option<HashSet<&str>>,
    property_mapping: &mut PropertyMapping,
) -> Result<PropertySchema, Box<dyn std::error::Error>> {
    let mut prompt_individual = Individual::default();
    if module.backend.storage.get_individual(system_prompt_id, &mut prompt_individual) != ResultCode::Ok {
        return Err("Failed to load prompt".into());
    }
    let properties = prompt_individual.get_literals("v-bpa:properties").unwrap_or_default();
    info!("@A0 properties={:?}", properties);

    // Собираем определения свойств
    let schema = collect_define_from_schema(module, properties, excluded, property_mapping);

    info!("@A1 schema={:?}", schema);

    Ok(schema)
}

/// Подготавливает параметры запроса для оптимизации на основе промпта из онтологии
pub fn prepare_request_ai_parameters(
    module: &mut BusinessProcessAnalysisModule,
    system_prompt_id: &str,
    analysis_data: Value,
    properties_schema: PropertySchema,
    property_mapping: &mut PropertyMapping,
) -> Result<ChatCompletionParameters, Box<dyn std::error::Error>> {
    let mut prompt_individual = Individual::default();
    if module.backend.storage.get_individual(system_prompt_id, &mut prompt_individual) != ResultCode::Ok {
        return Err("Failed to load prompt".into());
    }
    let prompt_text = prompt_individual.get_first_literal("v-bpa:promptText").ok_or("Prompt text not found")?;

    // Собираем имена свойств для списка required
    let required: Vec<String> = property_mapping
        .keys()
        .filter(|k| !k.contains('*')) // Исключаем маппинги для enum значений
        .cloned()
        .collect();

    // Формируем полную схему
    let schema = serde_json::json!({
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "result": {
                "type": "object",
                "additionalProperties": false,
                "properties": properties_schema,
                "required": required
            }
        },
        "required": ["result"]
    });

    info!("@A4 property_mapping={:?}", property_mapping);
    info!("@A5 schema={}", schema.to_string());

    let parameters = ChatCompletionParametersBuilder::default()
        .model(module.default_model.clone())
        .messages(vec![
            ChatMessage::System {
                content: ChatMessageContent::Text("You must respond only in Russian language. Use only Russian for all text fields.".to_string()),
                name: None,
            },
            ChatMessage::System {
                content: ChatMessageContent::Text(prompt_text),
                name: None,
            },
            ChatMessage::User {
                content: ChatMessageContent::Text(analysis_data.to_string()),
                name: None,
            },
        ])
        .response_format(ChatCompletionResponseFormat::JsonSchema(JsonSchemaBuilder::default().name("process_optimization").schema(schema).strict(true).build()?))
        .build()?;

    Ok(parameters)
}

pub async fn send_structured_request_to_ai(
    module: &mut BusinessProcessAnalysisModule,
    parameters: ChatCompletionParameters,
    client_type: ClientType,
) -> Result<AIResponseValues, Box<dyn std::error::Error>> {
    // Выбираем нужный клиент в зависимости от переданного типа
    let result = match client_type {
        ClientType::Default => module.default_client.chat().create(parameters).await?,
        ClientType::Reasoning => module.reasoning_client.chat().create(parameters).await?,
    };

    if let Some(usage) = result.usage {
        info!(
            "API usage metrics - Tokens: input={}, output={}, total={}, cost={}$",
            usage.prompt_tokens,
            usage.completion_tokens.unwrap_or(0),
            usage.total_tokens,
            calculate_cost(usage.total_tokens as f64, &module.default_model) // можно добавить логику выбора модели
        );
    }

    if let Some(choice) = result.choices.first() {
        if let ChatMessage::Assistant {
            content: Some(ChatMessageContent::Text(text)),
            ..
        } = &choice.message
        {
            let response: Value = serde_json::from_str(text)?;
            info!("@ response text ={}", text);
            let response_object = response.as_object().ok_or("Response is not a JSON object")?;
            let response_values: AIResponseValues = response_object.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
            Ok(response_values)
        } else {
            error!("Unexpected message format in AI response");
            Err("Unexpected message format".into())
        }
    } else {
        error!("No response received from AI");
        Err("No response from AI".into())
    }
}

/// Сохраняет результаты запроса к AI в индивид
///
/// # Arguments
/// * `module` - Модуль анализа с настройками и доступом к хранилищу
/// * `individual_id` - Идентификатор индивида для обновления
/// * `ai_response` - Значения из ответа AI
/// * `property_mapping` - Маппинг свойств (короткие имена -> URI)
///
/// # Returns
/// * `Result<(), Box<dyn std::error::Error>>` - Результат сохранения
pub fn set_to_individual_from_ai_response(
    module: &mut BusinessProcessAnalysisModule,
    individual: &mut Individual,
    ai_response: &AIResponseValues,
    property_mapping: &PropertyMapping,
) -> Result<(), Box<dyn std::error::Error>> {
    // Получаем вложенный объект optimized_process
    let response_values = if let Some(res) = ai_response.get("result") {
        if let Some(obj) = res.as_object() {
            obj
        } else {
            error!("result is not an object");
            return Err("result is not an object".into());
        }
    } else {
        error!("No result object found in AI response");
        return Err("Missing result object".into());
    };

    info!("@D response_values={:?}", response_values);

    for (short_name, value) in response_values {
        if let Some(full_prop) = property_mapping.get(short_name) {
            // Загружаем определение свойства
            let mut prop_individual = Individual::default();
            if module.backend.storage.get_individual(full_prop, &mut prop_individual) != ResultCode::Ok {
                warn!("Failed to load property definition for {}", full_prop);
                continue;
            }
            prop_individual.parse_all();

            let range = prop_individual.get_first_literal("rdfs:range").unwrap_or_default();

            // Очищаем предыдущие значения свойства
            individual.remove(full_prop);

            if !range.starts_with("xsd:") {
                // Обработка значений-ссылок
                if let Some(arr) = value.as_array() {
                    for val in arr {
                        if let Some(str_val) = val.as_str() {
                            let enum_key = format!("{}*{}", short_name, str_val);
                            if let Some(uri) = property_mapping.get(&enum_key) {
                                info!("Adding enum value {} -> {} for property {}", str_val, uri, full_prop);
                                individual.add_uri(full_prop, uri);
                            } else {
                                info!("Adding value {} for property {}", str_val, full_prop);
                                individual.add_string(full_prop, str_val, Lang::none());
                            }
                        }
                    }
                } else if let Some(str_val) = value.as_str() {
                    let enum_key = format!("{}*{}", short_name, str_val);
                    if let Some(uri) = property_mapping.get(&enum_key) {
                        info!("Setting enum value {} -> {} for property {}", str_val, uri, full_prop);
                        individual.set_uri(full_prop, uri);
                    } else {
                        info!("Setting value {} for property {}", str_val, full_prop);
                        individual.set_string(full_prop, str_val, Lang::none());
                    }
                }
            } else {
                // Обработка xsd:* типов
                match range.as_str() {
                    "xsd:string" => {
                        if let Some(arr) = value.as_array() {
                            for val in arr {
                                if let Some(str_val) = val.as_str() {
                                    individual.add_string(full_prop, str_val, Lang::none());
                                }
                            }
                        } else if let Some(str_val) = value.as_str() {
                            individual.set_string(full_prop, str_val, Lang::none());
                        }
                    },
                    "xsd:integer" => {
                        if let Some(arr) = value.as_array() {
                            for val in arr {
                                if let Some(num_val) = val.as_i64() {
                                    individual.add_integer(full_prop, num_val);
                                }
                            }
                        } else if let Some(num_val) = value.as_i64() {
                            individual.set_integer(full_prop, num_val);
                        }
                    },
                    "xsd:decimal" => {
                        if let Some(arr) = value.as_array() {
                            for val in arr {
                                if let Some(num_val) = val.as_f64() {
                                    individual.add_decimal_from_f64(full_prop, num_val);
                                }
                            }
                        } else if let Some(num_val) = value.as_f64() {
                            individual.add_decimal_from_f64(full_prop, num_val);
                        }
                    },
                    _ => {
                        warn!("Unknown range type {} for property {}, treating as string", range, full_prop);
                        if let Some(str_val) = value.as_str() {
                            individual.set_string(full_prop, str_val, Lang::none());
                        }
                    },
                }
            }
        } else {
            warn!("Property mapping not found for short name: {}", short_name);
        }
    }

    info!("Successfully set AI analysis results for individual {}", individual.get_id());
    Ok(())
}

pub fn format_time(seconds: i64) -> String {
    let duration = Duration::from_secs(seconds.unsigned_abs() as u64);
    format_duration(duration).to_string()
}

pub fn collect_define_from_schema(
    module: &mut BusinessProcessAnalysisModule,
    properties: Vec<String>,
    excluded: Option<HashSet<&str>>,
    property_mapping: &mut PropertyMapping,
) -> PropertySchema {
    let mut properties_defs = Vec::new();

    for full_prop in properties {
        let mut prop_individual = Individual::default();
        if module.backend.storage.get_individual(&full_prop, &mut prop_individual) != ResultCode::Ok {
            continue;
        }
        prop_individual.parse_all();

        let is_functional_property = prop_individual.any_exists("rdf:type", &["owl:FunctionalProperty"]);
        let range = prop_individual.get_first_literal("rdfs:range").unwrap_or_default();
        let description = prop_individual.get_first_literal_with_lang("rdfs:label", &[Lang::new_from_i64(1)]).unwrap_or_else(|| full_prop.clone());

        let short_name = full_prop.split(':').last().unwrap_or(&*full_prop).to_string();
        property_mapping.insert(short_name.clone(), full_prop.clone());

        let property_def = if !range.starts_with("xsd:") {
            info!("@A2 Processing class range: {} for property {}", range, full_prop);

            match get_individuals_by_type(module, &range) {
                Ok(mut instances) => {
                    let enum_values = instances
                        .iter_mut()
                        .filter(|instance| {
                            if let Some(ex) = &excluded {
                                !ex.contains(instance.get_id())
                            } else {
                                true
                            }
                        })
                        .filter_map(|instance| {
                            let label = instance.get_first_literal_with_lang("rdfs:label", &[Lang::new_from_i64(1)]);
                            if let Some(label) = &label {
                                property_mapping.insert(format!("{}*{}", short_name, label), instance.get_id().to_string());
                            }
                            label
                        })
                        .collect::<Vec<_>>();

                    info!("@A3 Found enum values for {}: {:?}", full_prop, enum_values);

                    if !enum_values.is_empty() {
                        if is_functional_property {
                            serde_json::json!({
                                short_name: {
                                    "type": "string",
                                    "enum": enum_values,
                                    "description": description
                                }
                            })
                        } else {
                            serde_json::json!({
                                short_name: {
                                    "type": "array",
                                    "items": {
                                        "type": "string",
                                        "enum": enum_values
                                    },
                                    "description": description
                                }
                            })
                        }
                    } else {
                        if is_functional_property {
                            serde_json::json!({
                                short_name: {
                                    "type": "string",
                                    "description": description
                                }
                            })
                        } else {
                            serde_json::json!({
                                short_name: {
                                    "type": "array",
                                    "items": {"type": "string"},
                                    "description": description
                                }
                            })
                        }
                    }
                },
                Err(e) => {
                    error!("Failed to get instances of type {}: {:?}", range, e);
                    if is_functional_property {
                        serde_json::json!({
                            short_name: {
                                "type": "string",
                                "description": description
                            }
                        })
                    } else {
                        serde_json::json!({
                            short_name: {
                                "type": "array",
                                "items": {"type": "string"},
                                "description": description
                            }
                        })
                    }
                },
            }
        } else {
            let item_type = match range.as_str() {
                "xsd:string" => "string",
                "xsd:integer" => "integer",
                "xsd:decimal" => "number",
                _ => "string",
            };

            if is_functional_property {
                serde_json::json!({
                    short_name: {
                        "type": item_type,
                        "description": description
                    }
                })
            } else {
                serde_json::json!({
                    short_name: {
                        "type": "array",
                        "items": {"type": item_type},
                        "description": description
                    }
                })
            }
        };

        properties_defs.push(property_def);
    }

    // Собираем все свойства в один объект
    let mut properties_obj = PropertySchema::new();
    for prop_def in properties_defs {
        properties_obj.extend(prop_def.as_object().unwrap().clone());
    }

    properties_obj
}

////////////////
/// Преобразует полные URI в человекочитаемые значения
fn transform_uri_to_display_value(uri: &str, property_mapping: &PropertyMapping) -> Option<String> {
    info!("Transforming URI to display value: {}", uri);
    for (key, value) in property_mapping {
        if value == uri {
            if let Some((_prefix, display_value)) = key.split_once('*') {
                info!("Found display value: {} for URI: {}", display_value, uri);
                return Some(display_value.to_string());
            }
        }
    }
    info!("No display value found for URI: {}", uri);
    None
}

/// Преобразует человекочитаемое значение обратно в URI
fn transform_display_value_to_uri(predicate: &str, display_value: &str, property_mapping: &PropertyMapping) -> Option<String> {
    let enum_key = format!("{}*{}", predicate, display_value);
    info!("Looking up URI for key: {}", enum_key);
    if let Some(uri) = property_mapping.get(&enum_key) {
        info!("Found URI: {} for display value: {}", uri, display_value);
        Some(uri.clone())
    } else {
        info!("No URI found for display value: {} (key: {})", display_value, enum_key);
        None
    }
}

/// Преобразует полные URI предикатов в короткие имена и их значения в человекочитаемый формат
pub fn convert_full_to_short_predicates(input: &Value, property_mapping: &mut PropertyMapping) -> Result<Value, Box<dyn std::error::Error>> {
    match input {
        Value::Object(map) => {
            let mut new_map = serde_json::Map::new();
            for (full_predicate, value) in map {
                let short_predicate = shorten_predicate_name(full_predicate, property_mapping);

                // Преобразуем значение
                let transformed_value = match value {
                    Value::Array(arr) => {
                        let transformed_arr: Vec<Value> = arr
                            .iter()
                            .map(|v| {
                                if let Value::String(uri) = v {
                                    if let Some(display_value) = transform_uri_to_display_value(uri, property_mapping) {
                                        Value::String(display_value)
                                    } else {
                                        v.clone()
                                    }
                                } else {
                                    v.clone()
                                }
                            })
                            .collect();
                        Value::Array(transformed_arr)
                    },
                    Value::String(uri) => {
                        if let Some(display_value) = transform_uri_to_display_value(uri, property_mapping) {
                            Value::String(display_value)
                        } else {
                            value.clone()
                        }
                    },
                    _ => value.clone(),
                };

                new_map.insert(short_predicate, transformed_value);
            }
            Ok(Value::Object(new_map))
        },
        _ => Err("Expected object for input".into()),
    }
}

/// Преобразует короткие имена предикатов в полные URI и их значения обратно в URI
pub fn convert_short_to_full_predicates(input: &Value, property_mapping: &PropertyMapping) -> Result<Value, Box<dyn std::error::Error>> {
    match input {
        Value::Object(map) => {
            let mut new_map = serde_json::Map::new();
            for (short_predicate, value) in map {
                let full_predicate = if let Some(full_name) = property_mapping.get(short_predicate) {
                    full_name.clone()
                } else {
                    short_predicate.clone()
                };

                // Преобразуем значение
                let transformed_value = match value {
                    Value::Array(arr) => {
                        let transformed_arr: Vec<Value> = arr
                            .iter()
                            .map(|v| {
                                if let Value::String(display_value) = v {
                                    if let Some(uri) = transform_display_value_to_uri(short_predicate, display_value, property_mapping) {
                                        Value::String(uri)
                                    } else {
                                        v.clone()
                                    }
                                } else {
                                    v.clone()
                                }
                            })
                            .collect();
                        Value::Array(transformed_arr)
                    },
                    Value::String(display_value) => {
                        if let Some(uri) = transform_display_value_to_uri(short_predicate, display_value, property_mapping) {
                            Value::String(uri)
                        } else {
                            value.clone()
                        }
                    },
                    _ => value.clone(),
                };

                new_map.insert(full_predicate, transformed_value);
            }
            Ok(Value::Object(new_map))
        },
        _ => Err("Expected object for input".into()),
    }
}

/// Преобразует полное URI предиката в короткое имя
fn shorten_predicate_name(full_name: &str, property_mapping: &mut PropertyMapping) -> String {
    let short_name = full_name.split(':').last().unwrap_or(full_name).to_string();
    if !property_mapping.contains_key(&short_name) {
        property_mapping.insert(short_name.clone(), full_name.to_string());
    }
    short_name
}

// Helper function to calculate cost based on model and tokens
pub fn calculate_cost(tokens: f64, model: &str) -> f64 {
    match model {
        // GPT-4 pricing
        "gpt-4-turbo-preview" => (tokens * 0.01) / 1000.0, // $0.01 per 1K tokens
        "gpt-4" => (tokens * 0.03) / 1000.0,               // $0.03 per 1K tokens

        // GPT-3.5 pricing
        "gpt-3.5-turbo-0125" => (tokens * 0.0015) / 1000.0, // $0.0015 per 1K tokens
        "gpt-3.5-turbo" => (tokens * 0.002) / 1000.0,       // $0.002 per 1K tokens

        // Default case
        _ => 0.0,
    }
}

/// Sends request to AI and gets text response
pub async fn send_text_request_to_ai(
    module: &mut BusinessProcessAnalysisModule,
    parameters: ChatCompletionParameters,
    client_type: ClientType,
) -> Result<String, Box<dyn std::error::Error>> {
    // Используем нужный клиент в зависимости от типа
    let result = match client_type {
        ClientType::Default => module.default_client.chat().create(parameters).await?,
        ClientType::Reasoning => module.reasoning_client.chat().create(parameters).await?,
    };

    if let Some(usage) = result.usage {
        info!(
            "API usage metrics - Tokens: input={}, output={}, total={}, cost={}$",
            usage.prompt_tokens,
            usage.completion_tokens.unwrap_or(0),
            usage.total_tokens,
            calculate_cost(usage.total_tokens as f64, &module.default_model)
        );
    }

    if let Some(choice) = result.choices.first() {
        if let ChatMessage::Assistant {
            content: Some(ChatMessageContent::Text(text)),
            ..
        } = &choice.message
        {
            Ok(text.clone())
        } else {
            error!("Unexpected message format in AI response");
            Err("Unexpected message format".into())
        }
    } else {
        error!("No response received from AI");
        Err("No response from AI".into())
    }
}
