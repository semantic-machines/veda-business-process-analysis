/// Обработчик для выполнения произвольных операций с индивидами на основе пользовательского ввода
/// и заданного типа целевого индивида.
use crate::common::{
    convert_full_to_short_predicates, convert_short_to_full_predicates, load_schema, prepare_request_ai_parameters, set_to_individual_from_ai_response, ClientType,
};
use crate::extractors::extract_text_from_document;
use crate::queue_processor::BusinessProcessAnalysisModule;
use crate::response_schema::ResponseSchema;
use crate::types::PropertyMapping;
use openai_dive::v1::resources::chat::{
    ChatCompletionParametersBuilder, ChatCompletionResponseFormat, ChatMessage, ChatMessageContent, ChatMessageContentPart, ChatMessageImageContentPart,
    ChatMessageTextContentPart, ImageUrlDetail, ImageUrlType, JsonSchemaBuilder,
};

use crate::ai_client::{save_to_interaction_file, send_structured_request_to_ai};
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

/// Process ontology input and create result individual
fn process_ontology_input(
    module: &mut BusinessProcessAnalysisModule,
    request: &mut Individual,
    prompt_individual: &mut Individual,
) -> Result<(), Box<dyn std::error::Error>> {
    // Get user input
    let raw_input = request.get_first_literal("v-bpa:rawInput").ok_or("No raw input provided")?;

    // Get target individual type
    let target_type = prompt_individual.get_first_literal("v-bpa:targetType").ok_or("No target type specified")?;

    // Load target type definition from ontology
    let mut target_type_def = Individual::default();
    if module.backend.storage.get_individual(&target_type, &mut target_type_def) != ResultCode::Ok {
        return Err(format!("Failed to load target type definition: {}", target_type).into());
    }
    target_type_def.parse_all();

    let mut property_mapping = PropertyMapping::new();
    let property_schema = load_schema(module, &prompt_individual.get_id(), None, &mut property_mapping)?;

    // Process input data if available
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

    // Prepare data for analysis
    let analysis_data = prepare_analysis_data(&raw_input, &mut target_type_def, structured_input)?;

    // Create request parameters and get property mapping
    let req_to_ai = prepare_request_ai_parameters(module, &prompt_individual.get_id(), analysis_data, property_schema, &mut property_mapping)?;

    save_to_interaction_file(&serde_json::to_string_pretty(&req_to_ai)?, "request", "json")?;

    // Send request to AI
    info!("Sending request to AI for processing input: {}", raw_input);
    let rt = Runtime::new()?;
    let ai_response = rt.block_on(async { send_structured_request_to_ai(module, req_to_ai, ClientType::Default).await })?;

    save_to_interaction_file(&serde_json::to_string_pretty(&ai_response)?, "response", "json")?;

    if is_structured_input {
        if let Some(result) = ai_response.get("result") {
            // Convert short names and human-readable values back to URIs
            let mapped_result = convert_short_to_full_predicates(result, &property_mapping)?;
            request.set_string("v-bpa:structuredOutput", &mapped_result.to_string(), Lang::none());
        }
    } else {
        // Create new result individual
        let result_id = format!("d:generic_result_{}", uuid::Uuid::new_v4());
        let mut result_individual = Individual::default();
        result_individual.set_id(&result_id);
        result_individual.set_uri("rdf:type", "v-bpa:GenericProcessingResult");
        result_individual.set_uri("v-bpa:targetType", &target_type);

        // Сохраняем результат анализа AI, включая очищенный текст
        set_to_individual_from_ai_response(module, &mut result_individual, &ai_response, &property_mapping)?;

        // Save updated individual
        if let Err(e) = module.backend.mstorage_api.update_or_err(&module.ticket, "", "BPA", IndvOp::Put, &mut result_individual) {
            error!("Failed to update individual {}: {:?}", result_individual.get_id(), e);
            return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("Failed to update individual, err={:?}", e))));
        }

        // Update original request with reference to created result
        request.set_uri("v-bpa:hasResult", &result_id);
    }

    request.set_uri("v-bpa:processingStatus", "v-bpa:Completed");

    if let Err(e) = module.backend.mstorage_api.update_or_err(&module.ticket, "", "BPA", IndvOp::Put, request) {
        error!("Failed to update request {}: {:?}", request.get_id(), e);
        return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("Failed to update request, err={:?}", e))));
    }
    Ok(())
}

/// Process structured schema data with support for separated or combined results
fn process_structured_schema(
    module: &mut BusinessProcessAnalysisModule,
    request: &mut Individual,
    prompt_individual: &mut Individual,
) -> Result<(), Box<dyn std::error::Error>> {
    // Get file extension and content from either attachment or raw input
    let (extension, extracted_contents) = if let Some(attachment_id) = request.get_first_literal("v-s:attachment") {
        // Load attachment individual
        let mut attachment = Individual::default();
        if module.backend.storage.get_individual(&attachment_id, &mut attachment) != ResultCode::Ok {
            error!("Failed to load attachment {}", attachment_id);
            return Err(format!("Failed to load attachment {}", attachment_id).into());
        }

        // Get file extension
        let extension =
            attachment.get_first_literal("v-s:fileName").ok_or("No filename in attachment")?.split('.').last().ok_or("Invalid filename format")?.to_lowercase();

        info!("Processing attachment with extension: {}", extension);

        // Get file URI and read content
        let file_uri = attachment.get_first_literal("v-s:fileUri").ok_or("No file URI in attachment")?;
        let file_path = attachment.get_first_literal("v-s:filePath").ok_or("No file path in attachment")?;
        let full_path = format!("./data/files/{}/{}", file_path, file_uri);
        info!("Reading file from path: {}", full_path);

        // Check if file exists
        if !Path::new(&full_path).exists() {
            error!("File does not exist: {}", full_path);
            return Err("File not found".into());
        }

        // Read file content
        let content = fs::read(&full_path).map_err(|e| {
            error!("Failed to read file: {}", e);
            format!("Failed to read file {}: {}", full_path, e)
        })?;

        (extension.clone(), extract_text_from_document(&content, &extension)?)
    } else {
        // No attachment - use raw input
        let raw_input = request.get_first_literal("v-bpa:rawInput").ok_or("Neither attachment nor raw input provided")?;
        ("txt".to_string(), vec![raw_input])
    };

    // Parse schema
    let response_schema = prompt_individual.get_first_literal("v-bpa:responseSchema").ok_or("No response schema found")?;
    let mut schema = ResponseSchema::from_json(&response_schema)?;
    let ai_schema = schema.to_ai_schema(module)?;

    let prompt_text = prompt_individual.get_first_literal("v-bpa:promptText").ok_or("Prompt text not found")?;

    // Generate base UUID for all results
    let base_result_id = format!("d:result_{}", uuid::Uuid::new_v4());

    // Check if results should be separated
    let separate_results = request.get_first_bool("v-bpa:separateResults").unwrap_or(false);

    if !separate_results {
        // Create initial result individual with type
        let mut initial_res = Individual::default();
        initial_res.set_id(&base_result_id);
        initial_res.set_uri("rdf:type", "v-bpa:GenericProcessingResult");
        initial_res.set_uri("v-s:hasParentLink", request.get_id());

        // Save empty individual with type
        if let Err(e) = module.backend.mstorage_api.update_or_err(&module.ticket, "", "BPA", IndvOp::Put, &mut initial_res) {
            error!("Failed to create initial result individual: {:?}", e);
            return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("Failed to create initial result individual: {:?}", e))));
        }

        request.add_uri("v-bpa:hasResult", &base_result_id);
    }

    // Process each extracted content
    for (index, base64_content) in extracted_contents.iter().enumerate() {
        let user_content = match extension.as_str() {
            "jpg" | "jpeg" | "pdf" => {
                vec![ChatMessageContentPart::Image(ChatMessageImageContentPart {
                    r#type: "image_url".to_string(),
                    image_url: ImageUrlType {
                        url: format!("data:image/jpeg;base64,{}", base64_content),
                        detail: Some(ImageUrlDetail::High),
                    },
                })]
            },
            _ => vec![ChatMessageContentPart::Text(ChatMessageTextContentPart {
                r#type: "text".to_string(),
                text: base64_content.to_string(),
            })],
        };

        let messages = vec![
            ChatMessage::System {
                content: ChatMessageContent::Text("You must respond only in Russian language. Use only Russian for all text fields.".to_string()),
                name: None,
            },
            ChatMessage::System {
                content: ChatMessageContent::Text(prompt_text.clone()),
                name: None,
            },
            ChatMessage::User {
                content: ChatMessageContent::ContentPart(user_content),
                name: None,
            },
        ];

        let parameters = ChatCompletionParametersBuilder::default()
            .seed(43 as u32)
            .model(module.default_model.clone())
            .max_tokens(16384 as u32)
            .messages(messages)
            .response_format(ChatCompletionResponseFormat::JsonSchema(
                JsonSchemaBuilder::default().name("document_analysis").schema(ai_schema.clone()).strict(true).build()?,
            ))
            .build()?;

        // Send request to AI
        //info!("Sending request to AI for analyzing content {}", index + 1);
        let rt = Runtime::new()?;
        let ai_response = rt.block_on(async { send_structured_request_to_ai(module, parameters, ClientType::Default).await })?;

        // Convert HashMap to Value and parse response
        let response_value = ai_response.to_json_value();
        let mut parse_result = schema.parse_ai_response(&response_value, module)?;

        if separate_results {
            // Generate result ID with sequence number for separate storage
            let result_id = format!("{}_{}", base_result_id, index + 1);
            parse_result.main_individual.set_id(&result_id);

            // Save individual immediately
            if let Err(e) = module.backend.mstorage_api.update_or_err(&module.ticket, "", "BPA", IndvOp::Put, &mut parse_result.main_individual) {
                error!("Failed to save individual {}: {:?}", result_id, e);
                return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("Failed to save individual, err={:?}", e))));
            }

            // Add result ID to request's hasResult
            request.add_uri("v-bpa:hasResult", &result_id);
        } else {
            // Add results to the main individual using AddTo operation
            let mut update = Individual::default();
            update.set_id(&base_result_id);

            // Copy all predicates from parse result except rdf:type
            for predicate in parse_result.main_individual.get_predicates() {
                if predicate != "rdf:type" {
                    update.apply_predicate_as_add_unique(&predicate, &mut parse_result.main_individual);
                }
            }

            // Update main individual using AddTo
            if let Err(e) = module.backend.mstorage_api.update_or_err(&module.ticket, "", "BPA", IndvOp::AddTo, &mut update) {
                error!("Failed to update main individual: {:?}", e);
                return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("Failed to update main individual: {:?}", e))));
            }
        }

        // Save related individuals immediately
        for mut related in parse_result.related_individuals {
            if let Err(e) = module.backend.mstorage_api.update_or_err(&module.ticket, "", "BPA", IndvOp::Put, &mut related) {
                error!("Failed to save related individual: {:?}", e);
                return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("Failed to save related individual: {:?}", e))));
            }
        }

        // Update processing status
        let status = if index == extracted_contents.len() - 1 {
            "v-bpa:Completed"
        } else {
            "v-bpa:Processing"
        };
        request.set_uri("v-bpa:processingStatus", status);

        // Update request status
        if let Err(e) = module.backend.mstorage_api.update_or_err(&module.ticket, "", "BPA", IndvOp::SetIn, request) {
            error!("Failed to update request status: {:?}", e);
            return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("Failed to update request status: {:?}", e))));
        }
    }

    // Final update of request
    if let Err(e) = module.backend.mstorage_api.update_or_err(&module.ticket, "", "BPA", IndvOp::Put, request) {
        error!("Failed to update request: {:?}", e);
        return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("Failed to update request: {:?}", e))));
    }

    Ok(())
}

/// Обработчик для выполнения произвольных операций с индивидами на основе пользовательского ввода
/// и заданного типа целевого индивида.
pub fn process_generic_request(module: &mut BusinessProcessAnalysisModule, request: &mut Individual) -> Result<(), Box<dyn std::error::Error>> {
    // Check processing status
    if request.any_exists("v-bpa:processingStatus", &["v-bpa:Completed"]) {
        return Ok(());
    }

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
