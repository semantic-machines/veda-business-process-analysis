use crate::queue_processor::BusinessProcessAnalysisModule;
use crate::types::{AIResponseValues, PropertyMapping};
use openai_dive::v1::resources::chat::{
    ChatCompletionParameters, ChatCompletionParametersBuilder, ChatCompletionResponseFormat, ChatMessage, ChatMessageContent, JsonSchemaBuilder,
};
use serde_json::Value;
use std::{io, thread, time};
use v_common::onto::datatype::Lang;
use v_common::onto::individual::Individual;
use v_common::search::common::{FTQuery, QueryResult};
use v_common::v_api::obj::ResultCode;

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
    let justification_refs = bp_obj.get_literals_nm("v-bpa:processJustification").unwrap_or_default();
    for ref_id in justification_refs {
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
        "justificationDocuments": documents
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

fn get_individuals_uris_by_query(module: &mut BusinessProcessAnalysisModule, query: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    info!("Starting search for individuals of query: {}", query);

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
        }
        retry_count += 1;
    }

    Ok(res.result)
}

/// Подготавливает параметры запроса для оптимизации на основе промпта из онтологии
pub fn prepare_request_ai_parameters(
    module: &mut BusinessProcessAnalysisModule,
    system_prompt_name: &str,
    analysis_data: Value,
) -> Result<(ChatCompletionParameters, PropertyMapping), Box<dyn std::error::Error>> {
    let mut prompt_individual = Individual::default();
    if module.backend.storage.get_individual(system_prompt_name, &mut prompt_individual) != ResultCode::Ok {
        return Err("Failed to load optimization prompt".into());
    }
    prompt_individual.parse_all();

    let prompt_text = prompt_individual.get_first_literal("v-bpa:promptText").ok_or("Prompt text not found")?;
    let properties = prompt_individual.get_literals("v-bpa:properties").unwrap_or_default();
    info!("@A1 properties={:?}", properties);

    // Словарь для хранения соответствия короткое имя -> полное имя
    let mut property_mapping = PropertyMapping::new();

    // Собираем определения свойств
    let mut properties_defs = Vec::new();

    for full_prop in properties {
        let mut prop_individual = Individual::default();
        if module.backend.storage.get_individual(&full_prop, &mut prop_individual) != ResultCode::Ok {
            continue;
        }
        prop_individual.parse_all();

        let range = prop_individual.get_first_literal("rdfs:range").unwrap_or_default();
        let description = prop_individual.get_first_literal_with_lang("rdfs:label", &[Lang::new_from_i64(1)]).unwrap_or_else(|| full_prop.clone());

        let short_name = full_prop.split(':').last().unwrap_or(&*full_prop).to_string();
        property_mapping.insert(short_name.clone(), full_prop.clone());

        let property_def = if !range.starts_with("xsd:") {
            info!("@A2 Processing class range: {} for property {}", range, full_prop);

            // Получаем все экземпляры этого класса
            let mut enum_values = Vec::new();

            match get_individuals_by_type(module, &range) {
                Ok(instances) => {
                    for mut instance in instances {
                        // Получаем метку на русском языке
                        if let Some(label) = instance.get_first_literal_with_lang("rdfs:label", &[Lang::new_from_i64(1)]) {
                            enum_values.push(label.clone());
                            // Сохраняем маппинг метка -> URI для этого значения
                            property_mapping.insert(format!("{}_{}", short_name, label), instance.get_id().to_string());
                        }
                    }

                    info!("@A3 Found enum values for {}: {:?}", full_prop, enum_values);

                    if enum_values.len() > 0 {
                        serde_json::json!({
                            short_name: {
                                "type": "string",
                                "description": description,
                                "enum": enum_values
                            }
                        })
                    } else {
                        serde_json::json!({
                            short_name: {
                                "type": "string",
                                "description": description,
                            }
                        })
                    }
                },
                Err(e) => {
                    error!("Failed to get instances of type {}: {:?}", range, e);
                    // Если не удалось получить экземпляры, обрабатываем как обычное строковое поле
                    serde_json::json!({
                        short_name: {
                            "type": "string",
                            "description": description
                        }
                    })
                },
            }
        } else {
            // Обрабатываем обычные xsd:* типы
            let json_type = match range.as_str() {
                "xsd:string" => "string",
                "xsd:integer" => "integer",
                "xsd:decimal" => "number",
                _ => "string",
            };

            serde_json::json!({
                short_name: {
                    "type": json_type,
                    "description": description
                }
            })
        };

        properties_defs.push(property_def);
    }

    // Собираем все свойства в один объект
    let mut properties_obj = serde_json::Map::new();
    for prop_def in properties_defs {
        properties_obj.extend(prop_def.as_object().unwrap().clone());
    }

    // Собираем имена свойств для списка required
    let required: Vec<String> = property_mapping
        .keys()
        .filter(|k| !k.contains('_')) // Исключаем маппинги для enum значений
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
                "properties": properties_obj,
                "required": required
            }
        },
        "required": ["result"]
    });

    info!("@A4 property_mapping={:?}", property_mapping);
    info!("@A5 schema={}", schema.to_string());

    let parameters = ChatCompletionParametersBuilder::default()
        .model(module.model.clone())
        .messages(vec![
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

    Ok((parameters, property_mapping))
}

/// Отправляет запрос к AI и обрабатывает ответ
///
/// # Arguments
/// * `module` - Модуль анализа с настройками и клиентом AI
/// * `parameters` - Параметры запроса к AI
///
/// # Returns
/// * `Result<AIResponseValues, Box<dyn std::error::Error>>` - Обработанный ответ от AI
pub async fn send_request_to_ai(
    module: &mut BusinessProcessAnalysisModule,
    parameters: ChatCompletionParameters,
) -> Result<AIResponseValues, Box<dyn std::error::Error>> {
    let result = module.client.chat().create(parameters).await?;

    if let Some(choice) = result.choices.first() {
        if let ChatMessage::Assistant {
            content: Some(ChatMessageContent::Text(text)),
            ..
        } = &choice.message
        {
            let response: Value = serde_json::from_str(text)?;
            let response_object = response.as_object().ok_or("Response is not a JSON object")?;

            // Преобразуем Map в HashMap
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
    let response_values = if let Some(optimized_process) = ai_response.get("result") {
        if let Some(obj) = optimized_process.as_object() {
            obj
        } else {
            error!("optimized_process is not an object");
            return Err("optimized_process is not an object".into());
        }
    } else {
        error!("No optimized_process object found in AI response");
        return Err("Missing optimized_process object".into());
    };

    for (short_name, value) in response_values {
        if let Some(full_prop) = property_mapping.get(short_name) {
            // Загружаем определение свойства из онтологии
            let mut prop_individual = Individual::default();
            if module.backend.storage.get_individual(full_prop, &mut prop_individual) != ResultCode::Ok {
                warn!("Failed to load property definition for {}", full_prop);
                continue;
            }
            prop_individual.parse_all();

            let range = prop_individual.get_first_literal("rdfs:range").unwrap_or_default();

            if !range.starts_with("xsd:") {
                // Обрабатываем значения-ссылки на экземпляры классов (enum)
                if let Some(str_val) = value.as_str() {
                    let enum_key = format!("{}_{}", short_name, str_val);
                    if let Some(uri) = property_mapping.get(&enum_key) {
                        info!("Setting enum value {} -> {} for property {}", str_val, uri, full_prop);
                        individual.set_uri(full_prop, uri);
                    } else {
                        info!("Setting value {} -> {} for property {}", enum_key, str_val, full_prop);
                        individual.set_string(full_prop, &str_val, Lang::none());
                    }
                }
            } else {
                // Обрабатываем xsd:* типы
                match range.as_str() {
                    "xsd:string" => {
                        if let Some(str_val) = value.as_str() {
                            individual.set_string(full_prop, str_val, Lang::none());
                        }
                    },
                    "xsd:integer" => {
                        if let Some(num_val) = value.as_i64() {
                            individual.set_integer(full_prop, num_val);
                        }
                    },
                    "xsd:decimal" => {
                        if let Some(num_val) = value.as_f64() {
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
