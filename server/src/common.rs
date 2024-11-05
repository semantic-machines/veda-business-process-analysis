use crate::queue_processor::BusinessProcessAnalysisModule;
use openai_dive::v1::resources::chat::{ChatCompletionParametersBuilder, ChatCompletionResponseFormat, ChatMessage, ChatMessageContent, JsonSchemaBuilder};
use serde_json::Value;
use v_common::onto::individual::Individual;
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

use crate::types::PropertyMapping;
use std::{io, thread, time};
use v_common::onto::datatype::Lang;
use v_common::search::common::{FTQuery, QueryResult};

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
///
/// # Arguments
/// * `module` - Модуль с настроенным подключением к базе
/// * `type_uri` - URI типа для поиска (например, "v-bpa:ProcessRelevance")
///
/// # Returns
/// * `Result<Vec<String>, Box<dyn std::error::Error>>` - Список uri найденных индивидов
pub fn get_individuals_uris_by_type(module: &mut BusinessProcessAnalysisModule, type_uri: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    info!("Starting search for individuals of type: {}", type_uri);

    let mut res = QueryResult::default();
    res.result_code = ResultCode::NotReady;

    let mut retry_count = 0;
    const MAX_RETRIES: u32 = 3;

    // Формируем запрос для поиска индивидов заданного типа
    while res.result_code == ResultCode::NotReady || res.result_code == ResultCode::DatabaseModifiedError {
        let query = format!("'rdf:type' === '{}'", type_uri);
        let ft_query = FTQuery::new_with_user("cfg:VedaSystem", &query);

        info!("Attempting to query individuals of type {} (attempt {})", type_uri, retry_count + 1);

        res = module.xr.query(ft_query, &mut module.backend.storage);

        if res.result_code == ResultCode::InternalServerError {
            error!("Search failed with internal server error");
            return Err(io::Error::new(io::ErrorKind::Other, format!("Search failed with error: {:?}", res.result_code)).into());
        }

        if res.result_code != ResultCode::Ok {
            if retry_count >= MAX_RETRIES {
                error!("Max retries reached while searching for type {}", type_uri);
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
pub fn prepare_optimization_parameters(
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
