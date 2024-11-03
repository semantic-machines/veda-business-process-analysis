use crate::common::extract_process_json;
use crate::prompt_manager::get_system_prompt;
use crate::queue_processor::BusinessProcessAnalysisModule;
use openai_dive::v1::resources::chat::{ChatCompletionParametersBuilder, ChatCompletionResponseFormat, ChatMessage, ChatMessageContent, JsonSchemaBuilder};
use serde_json;
use std::collections::HashMap;
use tokio::runtime::Runtime;
use v_common::onto::datatype::Lang;
use v_common::onto::individual::Individual;
use v_common::v_api::api_client::IndvOp;
use v_common::v_api::obj::ResultCode;

type OptimizedProcess = HashMap<String, serde_json::Value>;
type PropertyMapping = HashMap<String, String>; // short_name -> full_name

/// Анализирует кластер процессов и предлагает оптимизацию
pub fn analyze_and_optimize_cluster(module: &mut BusinessProcessAnalysisModule, cluster_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting cluster optimization analysis for cluster: {}", cluster_id);

    // Загружаем кластер
    let mut cluster = Individual::default();
    if module.backend.storage.get_individual(cluster_id, &mut cluster) != ResultCode::Ok {
        error!("Failed to load cluster {}", cluster_id);
        return Err(format!("Failed to load cluster {}", cluster_id).into());
    }

    // Получаем список процессов в кластере
    let process_ids = cluster.get_literals("v-bpa:hasProcess").unwrap_or_default();
    if process_ids.is_empty() {
        info!("No processes found in cluster {}", cluster_id);
        return Ok(());
    }

    // Загружаем данные всех процессов
    let mut processes_data = Vec::new();
    for process_id in &process_ids {
        let mut process = Individual::default();
        if module.backend.storage.get_individual(process_id, &mut process) != ResultCode::Ok {
            error!("Failed to load process {}", process_id);
            continue;
        }
        process.parse_all();
        if let Ok(data) = extract_process_json(&mut process, module) {
            processes_data.push(data);
        }
    }

    if processes_data.is_empty() {
        error!("No valid process data found in cluster {}", cluster_id);
        return Ok(());
    }

    // Подготавливаем данные для анализа
    info!("Preparing optimization data for {} processes", processes_data.len());
    let analysis_data = prepare_optimization_data(&processes_data)?;
    let system_prompt = get_system_prompt(module, "v-bpa:OptimizeProcessesPrompt")?;

    // Создаем параметры запроса и получаем маппинг свойств
    let (parameters, property_mapping) = prepare_optimization_parameters(module, system_prompt, analysis_data)?;

    // Отправляем запрос к AI
    info!("Sending optimization request to AI for cluster {}", cluster_id);
    let rt = Runtime::new()?;
    let optimization_result = rt.block_on(async { send_optimization_request(module, parameters).await })?;

    // Сохраняем результат оптимизации с учетом маппинга
    save_optimization_result(module, cluster_id, &optimization_result, &property_mapping)?;

    info!("Successfully completed optimization analysis for cluster {}", cluster_id);
    Ok(())
}

/// Подготавливает данные процессов для анализа оптимизации
fn prepare_optimization_data(processes: &[serde_json::Value]) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    Ok(serde_json::json!({
        "processes": processes,
        "count": processes.len()
    }))
}

/// Подготавливает параметры запроса для оптимизации на основе промпта из онтологии
fn prepare_optimization_parameters(
    module: &mut BusinessProcessAnalysisModule,
    system_prompt: String,
    analysis_data: serde_json::Value,
) -> Result<(openai_dive::v1::resources::chat::ChatCompletionParameters, PropertyMapping), Box<dyn std::error::Error>> {
    let mut prompt_individual = Individual::default();
    if module.backend.storage.get_individual("v-bpa:OptimizeProcessesPrompt", &mut prompt_individual) != ResultCode::Ok {
        return Err("Failed to load optimization prompt".into());
    }
    prompt_individual.parse_all();

    let properties = prompt_individual.get_literals("v-bpa:properties").unwrap_or_default();
    info!("@A1 properties={:?}", properties);

    // Словарь для хранения соответствия короткое имя -> полное имя
    let mut property_mapping = PropertyMapping::new();

    // Вектор для сбора определений свойств
    let mut property_definitions = Vec::new();
    let mut required = Vec::new();

    // Собираем определения свойств
    for full_prop in properties {
        let mut prop_individual = Individual::default();
        if module.backend.storage.get_individual(&full_prop, &mut prop_individual) != ResultCode::Ok {
            continue;
        }
        prop_individual.parse_all();

        let range = prop_individual.get_first_literal("rdfs:range").unwrap_or_default();
        let description = prop_individual.get_first_literal_with_lang("rdfs:label", &[Lang::new_from_i64(1)]).unwrap_or_else(|| full_prop.clone());

        let json_type = match range.as_str() {
            "xsd:string" => "string",
            "xsd:integer" => "integer",
            "xsd:decimal" => "number",
            _ => "string",
        };

        let short_name = full_prop.split(':').last().unwrap_or(&full_prop).to_string();
        property_mapping.insert(short_name.clone(), full_prop);
        required.push(short_name.clone());

        property_definitions.push((short_name, json_type, description));
    }

    info!("@A2 property_definitions={:?}", property_definitions);

    // Строим JSON строку для properties вручную, сохраняя порядок
    let mut properties_json = String::from("{\n");
    for (i, (name, type_name, description)) in property_definitions.iter().enumerate() {
        if i > 0 {
            properties_json.push_str(",\n");
        }
        properties_json.push_str(&format!(r#"    "{}": {{"type": "{}","description": "{}"}}"#, name, type_name, description));
    }
    properties_json.push_str("\n}");

    // Собираем полную схему с нашими упорядоченными properties
    let schema_str = format!(
        r#"{{
        "type": "object",
        "additionalProperties": false,
        "properties": {{
            "optimized_process": {{
                "type": "object",
                "additionalProperties": false,
                "properties": {},
                "required": {},
                "type": "object"
            }}
        }},
        "required": ["optimized_process"]
    }}"#,
        properties_json,
        serde_json::json!(required).to_string()
    );

    info!("@A3 schema_str={}", schema_str);

    let schema_value: serde_json::Value = serde_json::from_str(schema_str.as_str())?;

    let parameters = ChatCompletionParametersBuilder::default()
        .model(module.model.clone())
        .messages(vec![
            ChatMessage::System {
                content: ChatMessageContent::Text(system_prompt),
                name: None,
            },
            ChatMessage::User {
                content: ChatMessageContent::Text(analysis_data.to_string()),
                name: None,
            },
        ])
        .response_format(ChatCompletionResponseFormat::JsonSchema(
            JsonSchemaBuilder::default()
                .name("process_optimization")
                .schema(schema_value) // вызываем новый публичный метод schema()
                .strict(true)
                .build()?,
        ))
        .build()?;

    Ok((parameters, property_mapping))
}

/// Отправляет запрос на оптимизацию и получает результат
async fn send_optimization_request(
    module: &mut BusinessProcessAnalysisModule,
    parameters: openai_dive::v1::resources::chat::ChatCompletionParameters,
) -> Result<OptimizedProcess, Box<dyn std::error::Error>> {
    let result = module.client.chat().create(parameters).await?;

    if let Some(choice) = result.choices.first() {
        if let ChatMessage::Assistant {
            content: Some(ChatMessageContent::Text(text)),
            ..
        } = &choice.message
        {
            let response: serde_json::Value = serde_json::from_str(text)?;
            let optimized_map = response["optimized_process"].as_object().ok_or("Missing optimized_process object")?;

            // Правильное преобразование из Map в HashMap
            let optimized: OptimizedProcess = optimized_map.iter().map(|(k, v)| (k.clone(), v.clone())).collect();

            Ok(optimized)
        } else {
            error!("Unexpected message format in AI response");
            Err("Unexpected message format".into())
        }
    } else {
        error!("No response received from AI");
        Err("No response from AI".into())
    }
}

/// Сохраняет результат оптимизации
fn save_optimization_result(
    module: &mut BusinessProcessAnalysisModule,
    cluster_id: &str,
    optimization: &OptimizedProcess,
    property_mapping: &PropertyMapping,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut cluster = Individual::default();
    if module.backend.storage.get_individual(cluster_id, &mut cluster) != ResultCode::Ok {
        error!("Failed to load cluster {}", cluster_id);
        return Err(format!("Failed to load cluster {}", cluster_id).into());
    }
    cluster.parse_all();
    info!("Updating cluster {} with optimization results", cluster_id);

    for (short_name, value) in optimization {
        if let Some(full_prop) = property_mapping.get(short_name) {
            // Загружаем определение свойства из онтологии
            let mut prop_individual = Individual::default();
            if module.backend.storage.get_individual(full_prop, &mut prop_individual) != ResultCode::Ok {
                continue;
            }
            prop_individual.parse_all();

            let range = prop_individual.get_first_literal("rdfs:range").unwrap_or_default();

            // Сохраняем значение в соответствии с типом из онтологии
            match range.as_str() {
                "xsd:string" => {
                    if let Some(str_val) = value.as_str() {
                        cluster.set_string(full_prop, str_val, Lang::none());
                    }
                },
                "xsd:integer" => {
                    if let Some(num_val) = value.as_i64() {
                        cluster.set_integer(full_prop, num_val);
                    }
                },
                "xsd:decimal" => {
                    if let Some(num_val) = value.as_f64() {
                        cluster.add_decimal_from_f64(full_prop, num_val);
                    }
                },
                _ => {
                    if let Some(str_val) = value.as_str() {
                        cluster.set_string(full_prop, str_val, Lang::none());
                    }
                },
            }
        }
    }

    // Сохраняем обновленный кластер
    if let Err(e) = module.backend.mstorage_api.update_or_err(&module.ticket, "", "BPA", IndvOp::Put, &mut cluster) {
        error!("Failed to update cluster {}: {:?}", cluster_id, e);
        return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("Failed to update cluster, err={:?}", e))));
    }

    info!("Successfully saved optimization results for cluster {}", cluster_id);
    Ok(())
}
