/// Обработчик для выполнения произвольных операций с индивидами на основе пользовательского ввода
/// и заданного типа целевого индивида.
use crate::common::{
    convert_full_to_short_predicates, convert_short_to_full_predicates, load_schema, prepare_request_ai_parameters, send_request_to_ai,
    set_to_individual_from_ai_response,
};
use crate::extractors::extract_text_from_document;
use crate::queue_processor::BusinessProcessAnalysisModule;
use crate::response_schema::ResponseSchema;
use crate::types::PropertyMapping;
use openai_dive::v1::resources::assistant::message::MessageContent::ImageUrl;
use openai_dive::v1::resources::chat::{
    ChatCompletionParametersBuilder, ChatCompletionResponseFormat, ChatMessage, ChatMessageContent, ChatMessageContentPart, ChatMessageImageContentPart,
    ChatMessageTextContentPart, ImageUrlDetail, ImageUrlType, JsonSchemaBuilder,
};

use serde_json::Value;
use std::fs;
use std::path::Path;
use tokio::runtime::Runtime;
use v_common::onto::datatype::Lang;
use v_common::onto::individual::Individual;
use v_common::v_api::api_client::IndvOp;
use v_common::v_api::obj::ResultCode;

/// Подготавливает данные для анализа на основе пользовательского ввода,
/// определения целевого типа и промпта
fn prepare_analysis_data(raw_input: &str, target_type_def: &mut Individual, input_data: Option<Value>) -> Result<Value, Box<dyn std::error::Error>> {
    let mut data = serde_json::json!({
        "input": raw_input,
        "targetType": {
            "id": target_type_def.get_id(),
            "label": target_type_def.get_first_literal("rdfs:label")
        }
    });

    // Если есть дополнительные входные данные, добавляем их
    if let Some(input) = input_data {
        if let Some(obj) = data.as_object_mut() {
            obj.insert("inputData".to_string(), input);
        }
    }

    Ok(data)
}

fn process_ontology_input(
    module: &mut BusinessProcessAnalysisModule,
    request: &mut Individual,
    prompt_individual: &mut Individual,
) -> Result<(), Box<dyn std::error::Error>> {
    // Получаем пользовательский ввод
    let raw_input = request.get_first_literal("v-bpa:rawInput").ok_or("No raw input provided")?;

    // Получаем тип целевого индивида
    let target_type = prompt_individual.get_first_literal("v-bpa:targetType").ok_or("No target type specified")?;
    // Загружаем определение целевого типа из онтологии
    let mut target_type_def = Individual::default();
    if module.backend.storage.get_individual(&target_type, &mut target_type_def) != ResultCode::Ok {
        return Err(format!("Failed to load target type definition: {}", target_type).into());
    }
    target_type_def.parse_all();

    let mut property_mapping = PropertyMapping::new();
    let property_schema = load_schema(module, &prompt_individual.get_id(), None, &mut property_mapping)?;

    // Обрабатываем входные данные, если они есть
    let structured_input = if let Some(input_str) = request.get_first_literal("v-bpa:structuredInput") {
        info!("Processing additional input data: {}", input_str);
        let parsed_input: Value = serde_json::from_str(&input_str)?;
        let transformed_input = convert_full_to_short_predicates(&parsed_input, &mut property_mapping)?;
        Some(transformed_input)
    } else {
        None
    };

    let is_structured_input = structured_input.is_some();

    info!("@B structured_input={:?}", structured_input);
    // Подготавливаем данные для анализа
    let analysis_data = prepare_analysis_data(&raw_input, &mut target_type_def, structured_input)?;

    // Создаем параметры запроса и получаем маппинг свойств
    let req_to_ai = prepare_request_ai_parameters(module, &prompt_individual.get_id(), analysis_data, property_schema, &mut property_mapping)?;

    info!("@E = req_to_ai={:?}", req_to_ai);

    // Отправляем запрос к AI
    info!("Sending request to AI for processing input: {}", raw_input);
    let rt = Runtime::new()?;
    let ai_response = rt.block_on(async { send_request_to_ai(module, req_to_ai).await })?;

    info!("@G ai_response={:?}", ai_response);

    if is_structured_input {
        if is_structured_input {
            if let Some(result) = ai_response.get("result") {
                // Преобразуем короткие имена и человекочитаемые значения обратно в URI
                let mapped_result = convert_short_to_full_predicates(result, &property_mapping)?;
                request.set_string("v-bpa:structuredOutput", &mapped_result.to_string(), Lang::none());
            }
        }
    } else {
        // Создаем новый индивид целевого типа для сохранения результата
        let result_id = format!("d:generic_result_{}", uuid::Uuid::new_v4());
        let mut result_individual = Individual::default();
        result_individual.set_id(&result_id);
        result_individual.set_uri("rdf:type", "v-bpa:GenericProcessingResult");
        result_individual.set_uri("v-bpa:targetType", &target_type);

        // Сохраняем результат анализа AI, включая очищенный текст
        set_to_individual_from_ai_response(module, &mut result_individual, &ai_response, &property_mapping)?;

        // Сохраняем обновленный индивид
        if let Err(e) = module.backend.mstorage_api.update_or_err(&module.ticket, "", "BPA", IndvOp::Put, &mut result_individual) {
            error!("Failed to update individual {}: {:?}", result_individual.get_id(), e);
            return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("Failed to update individual, err={:?}", e))));
        }

        // Обновляем исходный запрос, добавляя ссылку на созданный результат
        request.set_uri("v-bpa:hasResult", &result_id);
    }

    request.set_uri("v-bpa:processingStatus", "v-bpa:Completed");

    if let Err(e) = module.backend.mstorage_api.update_or_err(&module.ticket, "", "BPA", IndvOp::Put, request) {
        error!("Failed to update request {}: {:?}", request.get_id(), e);
        return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("Failed to update request, err={:?}", e))));
    }
    Ok(())
}

use openai_dive::v1::resources::chat::ChatCompletionParameters;

fn process_structured_schema(
    module: &mut BusinessProcessAnalysisModule,
    request: &mut Individual,
    prompt_individual: &mut Individual,
) -> Result<(), Box<dyn std::error::Error>> {
    // Get content from file or raw input
    let content = if let Some(file_path) = request.get_first_literal("v-bpa:rawInputPath") {
        let path = Path::new(&file_path);
        info!("Trying to read file from path: {}", path.display());

        // Check if file exists
        if !path.exists() {
            error!("File does not exist: {}", path.display());
            return Err("File not found".into());
        }

        // Read file content
        fs::read(path).map_err(|e| {
            error!("Failed to read file: {}", e);
            format!("Failed to read file {}: {}", file_path, e)
        })?
    } else {
        let raw_input = request.get_first_literal("v-bpa:rawInput").ok_or("No raw input provided")?;
        raw_input.as_bytes().to_vec()
    };

    // Get file extension and process content accordingly
    let extension = if let Some(file_path) = request.get_first_literal("v-bpa:rawInputPath") {
        let path = Path::new(&file_path);
        path.extension().and_then(|ext| ext.to_str()).ok_or("File has no extension")?.to_lowercase()
    } else {
        "txt".to_string() // Default to text for raw input
    };

    info!("Processing file with extension: {}", extension);
    let extracted_content = extract_text_from_document(&content, &extension)?;

    // Parse schema
    let response_schema = prompt_individual.get_first_literal("v-bpa:responseSchema").ok_or("No response schema found")?;
    let schema = ResponseSchema::from_json(&response_schema)?;
    let ai_schema = schema.to_ai_schema()?;

    info!("Generated AI schema: {}", serde_json::to_string_pretty(&ai_schema)?);

    let prompt_text = prompt_individual.get_first_literal("v-bpa:promptText").ok_or("Prompt text not found")?;

    // Create content based on file type
    let user_content = match extension.as_str() {
        "jpg" | "jpeg" => vec![ChatMessageContentPart::Image(ChatMessageImageContentPart {
            r#type: "image_url".to_string(),
            image_url: ImageUrlType {
                url: format!("data:image/jpeg;base64,{}", extracted_content),
                detail: Some(ImageUrlDetail::High),
            },
        })],

        _ => {
            // TODO TEXT
            vec![ChatMessageContentPart::Image(ChatMessageImageContentPart {
                r#type: "image_url".to_string(),
                image_url: ImageUrlType {
                    url: format!("data:image/jpeg;base64,{}", extracted_content),
                    detail: Some(ImageUrlDetail::High),
                },
            })]
        },
    };

    let messages = vec![
        ChatMessage::System {
            content: ChatMessageContent::Text(prompt_text),
            name: None,
        },
        ChatMessage::User {
            content: ChatMessageContent::ContentPart(user_content),
            name: None,
        },
    ];

    // Build request parameters
    let parameters = ChatCompletionParametersBuilder::default()
        .model(module.model.clone())
        .messages(messages)
        .response_format(ChatCompletionResponseFormat::JsonSchema(JsonSchemaBuilder::default().name("document_analysis").schema(ai_schema).strict(true).build()?))
        .build()?;

    // Log the request parameters
    info!("Sending request to AI with parameters: {}", serde_json::to_string_pretty(&parameters)?);

    // Send request to AI
    info!("Sending request to AI for analyzing document");
    let rt = Runtime::new()?;
    let ai_response = rt.block_on(async { send_request_to_ai(module, parameters).await })?;

    info!("Received AI response: {:?}", ai_response);

    // Convert HashMap to Value
    let response_value = serde_json::to_value(&ai_response)?;
    info!("Converted AI response to Value: {}", serde_json::to_string_pretty(&response_value)?);

    // Parse response and create result
    let mut parse_result = schema.parse_ai_response(&response_value, &mut module.backend, &module.ticket)?;

    // Save main individual and get its ID
    let result_id = parse_result.main_individual.get_id().to_string();
    info!("Saving main individual with ID: {}", result_id);

    // Save main individual
    if let Err(e) = module.backend.mstorage_api.update_or_err(&module.ticket, "", "BPA", IndvOp::Put, &mut parse_result.main_individual) {
        error!("Failed to save main individual: {:?}", e);
        return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("Failed to save main individual: {:?}", e))));
    }

    // Save related individuals
    for mut related in parse_result.related_individuals {
        info!("Saving related individual: {}", related.get_id());
        if let Err(e) = module.backend.mstorage_api.update_or_err(&module.ticket, "", "BPA", IndvOp::Put, &mut related) {
            error!("Failed to save related individual: {:?}", e);
            return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("Failed to save related individual: {:?}", e))));
        }
    }

    // Update request with the result reference and status
    request.set_uri("v-bpa:hasResult", &result_id);
    request.set_uri("v-bpa:processingStatus", "v-bpa:Completed");

    if let Err(e) = module.backend.mstorage_api.update_or_err(&module.ticket, "", "BPA", IndvOp::Put, request) {
        error!("Failed to update request: {:?}", e);
        return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("Failed to update request: {:?}", e))));
    }

    Ok(())
}

/// Обработчик для выполнения произвольных операций с индивидами на основе пользовательского ввода
/// и заданного типа целевого индивида.
pub fn process_generic_request(module: &mut BusinessProcessAnalysisModule, request: &mut Individual) -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting generic request processing for request: {}", request.get_id());

    // Получаем ссылку на промпт и загружаем его
    let prompt_id = request.get_first_literal("v-bpa:prompt").ok_or("No prompt specified")?;

    let mut prompt_individual = Individual::default();
    if module.backend.storage.get_individual(&prompt_id, &mut prompt_individual) != ResultCode::Ok {
        return Err(format!("Failed to load prompt: {}", prompt_id).into());
    }

    if prompt_individual.is_exists("v-bpa:responseSchema") {
        process_structured_schema(module, request, &mut prompt_individual)?;
    } else {
        process_ontology_input(module, request, &mut prompt_individual)?;
    }

    info!("Successfully processed generic request {} ", request.get_id());
    Ok(())
}
