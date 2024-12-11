use crate::ai_client::send_structured_request_to_ai;
use crate::common::ClientType;
use crate::queue_processor::BusinessProcessAnalysisModule;
use crate::response_schema::ResponseSchema;
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use chrono::Utc;
use openai_dive::v1::resources::chat::{
    ChatCompletionParametersBuilder, ChatCompletionResponseFormat, ChatMessage, ChatMessageContent, ChatMessageContentPart, ChatMessageImageContentPart,
    ChatMessageTextContentPart, ImageUrlDetail, ImageUrlType, JsonSchemaBuilder,
};
use std::fs;
use std::path::Path;
use tokio::runtime::Runtime;
use v_common::onto::datatype::Lang;
use v_common::onto::individual::Individual;
use v_common::v_api::api_client::IndvOp;
use v_common::v_api::obj::ResultCode;

/// Internal implementation of structured schema processing
fn process_structured_schema_internal(
    module: &mut BusinessProcessAnalysisModule,
    request: &mut Individual,
    prompt_individual: &mut Individual,
) -> Result<(), Box<dyn std::error::Error>> {
    // Parse schema
    let response_schema = prompt_individual.get_first_literal("v-bpa:responseSchema").ok_or("No response schema found")?;
    let mut schema = ResponseSchema::from_json(&response_schema)?;
    let ai_schema = schema.to_ai_schema(module)?;

    let prompt_text = prompt_individual.get_first_literal("v-bpa:promptText").ok_or("Prompt text not found")?;

    // Get file extension and content from either attachment or raw input
    let (extension, extracted_contents) = if let Some(attachment_id) = request.get_first_literal("v-s:attachment") {
        // Load attachment individual
        let mut attachment = Individual::default();
        if module.backend.storage.get_individual(&attachment_id, &mut attachment) != ResultCode::Ok {
            return Err(format!("Failed to load attachment {}", attachment_id).into());
        }

        // Get file extension
        let extension = attachment.get_first_literal("v-s:fileUri").ok_or("No fileUri in attachment")?.split('.').last().ok_or("Invalid fileUri format")?.to_lowercase();

        info!("Processing attachment with extension: {}", extension);

        // Get file path and read content
        let file_uri = attachment.get_first_literal("v-s:fileUri").ok_or("No file URI in attachment")?;
        let file_path = attachment.get_first_literal("v-s:filePath").ok_or("No file path in attachment")?;
        let full_path = format!("{}/{}", file_path, file_uri);

        if !Path::new(&full_path).exists() {
            return Err(format!("File not found: {}", full_path).into());
        }

        let content = fs::read(&full_path)?;
        let encoded = STANDARD.encode(&content);

        (extension, encoded)
    } else {
        // Use raw input
        let raw_input = request.get_first_literal("v-bpa:rawInput").ok_or("Neither attachment nor raw input provided")?;
        ("txt".to_string(), raw_input)
    };

    // Prepare user content for AI request
    let user_content = vec![prepare_content_for_ai(&extension, extracted_contents)?];

    let messages = vec![
        ChatMessage::System {
            content: ChatMessageContent::Text("You must respond only in Russian language. Use only Russian for all text fields.".to_string()),
            name: None,
        },
        ChatMessage::System {
            content: ChatMessageContent::Text(prompt_text),
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
        .response_format(ChatCompletionResponseFormat::JsonSchema(JsonSchemaBuilder::default().name("document_analysis").schema(ai_schema).strict(true).build()?))
        .build()?;

    // Send request to AI
    info!("Sending request to AI for processing");
    let rt = Runtime::new()?;
    let ai_response = rt.block_on(async { send_structured_request_to_ai(module, parameters, ClientType::Default).await })?;

    // Process AI response
    let response_value = ai_response.to_json_value();
    let mut parse_result = schema.parse_ai_response(&response_value, module)?;

    // Create and save result
    let result_id = format!("d:result_{}", uuid::Uuid::new_v4());
    parse_result.main_individual.set_id(&result_id);

    // Save main individual
    if let Err(e) = module.backend.mstorage_api.update_or_err(&module.ticket, "", "BPA", IndvOp::Put, &mut parse_result.main_individual) {
        error!("Failed to save individual {}: {:?}", result_id, e);
        return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("Failed to save individual, err={:?}", e))));
    }

    // Save related individuals
    for mut related in parse_result.related_individuals {
        if let Err(e) = module.backend.mstorage_api.update_or_err(&module.ticket, "", "BPA", IndvOp::Put, &mut related) {
            error!("Failed to save related individual: {:?}", e);
            return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("Failed to save related individual: {:?}", e))));
        }
    }

    // Update request status
    request.set_uri("v-bpa:hasResult", &result_id);
    request.set_uri("v-bpa:processingStatus", "v-bpa:Completed");
    request.set_integer("v-bpa:percentComplete", 100);

    // Handle parent processing if needed
    if let Some(parent_id) = request.get_first_literal("v-s:hasParentLink") {
        trigger_parent_processing(module, &parent_id)?;
    }

    if let Err(e) = module.backend.mstorage_api.update_or_err(&module.ticket, "", "BPA", IndvOp::Put, request) {
        error!("Failed to update request: {:?}", e);
        return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("Failed to update request: {:?}", e))));
    }

    Ok(())
}

/// Prepare content for AI request
fn prepare_content_for_ai(format: &str, content: String) -> Result<ChatMessageContentPart, Box<dyn std::error::Error>> {
    match format {
        "txt" => Ok(ChatMessageContentPart::Text(ChatMessageTextContentPart {
            r#type: "text".to_string(),
            text: content,
        })),
        _ => Ok(ChatMessageContentPart::Image(ChatMessageImageContentPart {
            r#type: "image_url".to_string(),
            image_url: ImageUrlType {
                url: format!("data:image/{};base64,{}", format, content),
                detail: Some(ImageUrlDetail::High),
            },
        })),
    }
}

/// Trigger processing update for parent object
fn trigger_parent_processing(module: &mut BusinessProcessAnalysisModule, parent_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    info!("Triggering processing for parent object: {}", parent_id);

    let mut update = Individual::default();
    update.set_id(parent_id);
    update.set_datetime("v-s:modified", Utc::now().timestamp());

    if let Err(e) = module.backend.mstorage_api.update_or_err(&module.ticket, "", "trigger", IndvOp::SetIn, &mut update) {
        error!("Failed to trigger parent processing: {:?}", e);
        return Err(format!("Failed to trigger processing: {:?}", e).into());
    }

    Ok(())
}

/// Main entry point for structured schema processing
pub fn process_structured_schema(
    module: &mut BusinessProcessAnalysisModule,
    request: &mut Individual,
    prompt_individual: &mut Individual,
) -> Result<(), Box<dyn std::error::Error>> {
    // Execute main processing logic and handle any errors
    if let Err(e) = process_structured_schema_internal(module, request, prompt_individual) {
        error!("Processing failed: {:?}", e);

        // Set error status and details
        request.set_uri("v-bpa:processingStatus", "v-bpa:Failed");
        request.set_string("v-bpa:lastError", &e.to_string(), Lang::none());

        // Save error status
        if let Err(update_err) = module.backend.mstorage_api.update_or_err(&module.ticket, "", "BPA", IndvOp::SetIn, request) {
            error!("Failed to update request error status: {:?}", update_err);
            return Err(format!("Failed to update request: {:?}", update_err).into());
        }

        return Err(e);
    }

    Ok(())
}
