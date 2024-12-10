use crate::ai_client::send_structured_request_to_ai;
use crate::common::ClientType;
use crate::extractors::extract_text_from_document;
use crate::queue_processor::BusinessProcessAnalysisModule;
use crate::response_schema::ResponseSchema;
use chrono::Utc;
use openai_dive::v1::resources::chat::{
    ChatCompletionParametersBuilder, ChatCompletionResponseFormat, ChatMessage, ChatMessageContent, ChatMessageContentPart, ChatMessageImageContentPart,
    ChatMessageTextContentPart, ImageUrlDetail, ImageUrlType, JsonSchemaBuilder,
};
use std::fs;
use std::path::Path;
use tokio::runtime::Runtime;
use v_common::onto::individual::Individual;
use v_common::v_api::api_client::IndvOp;
use v_common::v_api::obj::ResultCode;

/// Process structured schema data with support for separated or combined results
pub fn process_structured_schema(
    module: &mut BusinessProcessAnalysisModule,
    request: &mut Individual,
    prompt_individual: &mut Individual,
) -> Result<(), Box<dyn std::error::Error>> {
    // Initial progress
    request.set_integer("v-bpa:percentComplete", 0);
    if let Err(e) = module.backend.mstorage_api.update_or_err(&module.ticket, "", "BPA", IndvOp::SetIn, request) {
        error!("Failed to set initial progress: {:?}", e);
        return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("Failed to set initial progress: {:?}", e))));
    }

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

    let extracted_contents_len = extracted_contents.len();

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

        // Calculate and update progress (0-100%)
        let progress = ((index + 1) as f64 / extracted_contents_len as f64 * 100.0) as i64;
        request.set_integer("v-bpa:percentComplete", progress);

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

        // Update request status and progress
        if let Err(e) = module.backend.mstorage_api.update_or_err(&module.ticket, "", "BPA", IndvOp::SetIn, request) {
            error!("Failed to update request status: {:?}", e);
            return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("Failed to update request status: {:?}", e))));
        }

        // Trigger parent processing to notify about progress update
        if let Err(e) = trigger_object_processing(module, request) {
            warn!("Failed to trigger object processing: {:?}", e);
        }

        if status == "v-bpa:Completed" {
            // Check if request has parent and notify completion
            if let Some(parent_link) = request.get_first_literal("v-s:hasParentLink") {
                let mut parent = Individual::default();
                if module.backend.storage.get_individual(&parent_link, &mut parent) == ResultCode::Ok {
                    info!("Updating parent with request completion: {}", parent_link);

                    // Create update to set request status in parent
                    let mut update = Individual::default();
                    update.set_id(&parent_link);
                    update.set_uri("v-bpa:hasCompletedRequest", request.get_id());

                    // Send update to parent
                    if let Err(e) = module.backend.mstorage_api.update_or_err(&module.ticket, "", "", IndvOp::SetIn, &mut update) {
                        error!("Failed to update parent with completion status: {:?}", e);
                    }
                }
            }
        }
    }

    // Final update of request
    request.set_integer("v-bpa:percentComplete", 100);
    if let Err(e) = module.backend.mstorage_api.update_or_err(&module.ticket, "", "BPA", IndvOp::Put, request) {
        error!("Failed to update request: {:?}", e);
        return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("Failed to update request: {:?}", e))));
    }

    // Trigger parent processing for final update
    if let Err(e) = trigger_object_processing(module, request) {
        warn!("Failed to trigger object processing: {:?}", e);
    }

    Ok(())
}

/// Triggers object processing by updating its modified timestamp
fn trigger_object_processing(module: &mut BusinessProcessAnalysisModule, request: &mut Individual) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent_id) = request.get_first_literal("v-s:hasParentLink") {
        info!("Triggering processing for object: {}", parent_id);

        // Create update
        let mut update = Individual::default();
        update.set_id(&parent_id);

        // Set modified datetime to trigger processing
        update.set_datetime("v-s:modified", Utc::now().timestamp());

        // Save update
        if let Err(e) = module.backend.mstorage_api.update_or_err(&module.ticket, "", "trigger", IndvOp::SetIn, &mut update) {
            error!("Failed to trigger object processing: {:?}", e);
            return Err(format!("Failed to trigger processing: {:?}", e).into());
        }
    }
    Ok(())
}
