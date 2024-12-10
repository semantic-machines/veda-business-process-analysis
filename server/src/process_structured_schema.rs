use crate::ai_client::send_structured_request_to_ai;
use crate::common::ClientType;
use crate::queue_processor::BusinessProcessAnalysisModule;
use crate::response_schema::ResponseSchema;
use chrono::Utc;
use openai_dive::v1::resources::chat::{ChatCompletionParametersBuilder, ChatCompletionResponseFormat, ChatMessage, ChatMessageContent, JsonSchemaBuilder};
use tokio::runtime::Runtime;
use v_common::onto::individual::Individual;
use v_common::v_api::api_client::IndvOp;

/// Process data using structured schema with support for separated or combined results
pub fn process_structured_schema(
    module: &mut BusinessProcessAnalysisModule,
    request: &mut Individual,
    prompt_individual: &mut Individual,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("=== Starting structured schema processing ===");
    info!("Processing request ID: {}", request.get_id());
    info!("Using prompt ID: {}", prompt_individual.get_id());

    // Parse schema
    let response_schema = prompt_individual.get_first_literal("v-bpa:responseSchema").ok_or("No response schema found")?;
    let mut schema = ResponseSchema::from_json(&response_schema)?;
    let ai_schema = schema.to_ai_schema(module)?;
    info!("Schema successfully parsed");

    // Get prompt and raw input
    let prompt_text = prompt_individual.get_first_literal("v-bpa:promptText").ok_or("Prompt text not found")?;
    let raw_input = request.get_first_literal("v-bpa:rawInput").ok_or("No raw input provided")?;
    info!("Input text length: {} characters", raw_input.len());

    // Process with AI
    info!("Preparing AI request parameters...");
    let parameters = prepare_ai_parameters(module, &prompt_text, &raw_input, ai_schema)?;
    info!("Sending request to AI...");
    let response = send_request_to_ai(module, parameters)?;
    info!("Received AI response");

    // Handle results
    info!("Processing AI response...");
    process_ai_response(module, request, response, &mut schema)?;

    info!("=== Successfully completed structured schema processing for request {} ===", request.get_id());
    Ok(())
}

/// Prepares parameters for AI request
fn prepare_ai_parameters(
    module: &mut BusinessProcessAnalysisModule,
    prompt_text: &str,
    raw_input: &str,
    ai_schema: serde_json::Value,
) -> Result<openai_dive::v1::resources::chat::ChatCompletionParameters, Box<dyn std::error::Error>> {
    let parameters = ChatCompletionParametersBuilder::default()
        .model(module.default_model.clone())
        .messages(vec![
            ChatMessage::System {
                content: ChatMessageContent::Text("You must respond only in Russian language. Use only Russian for all text fields.".to_string()),
                name: None,
            },
            ChatMessage::System {
                content: ChatMessageContent::Text(prompt_text.to_string()),
                name: None,
            },
            ChatMessage::User {
                content: ChatMessageContent::Text(raw_input.to_string()),
                name: None,
            },
        ])
        .response_format(ChatCompletionResponseFormat::JsonSchema(JsonSchemaBuilder::default().name("document_analysis").schema(ai_schema).strict(true).build()?))
        .build()?;

    Ok(parameters)
}

/// Sends request to AI and gets response
fn send_request_to_ai(
    module: &mut BusinessProcessAnalysisModule,
    parameters: openai_dive::v1::resources::chat::ChatCompletionParameters,
) -> Result<crate::ai_client::AIResponseValues, Box<dyn std::error::Error>> {
    info!("Creating runtime for async AI request...");
    let rt = Runtime::new()?;

    info!("Sending request to AI service...");
    let response = rt.block_on(async { send_structured_request_to_ai(module, parameters, ClientType::Default).await })?;

    if let Some(usage) = response.get("usage") {
        info!("AI request completed. Usage info: {:?}", usage);
    } else {
        info!("AI request completed without usage info");
    }

    Ok(response)
}

/// Processes AI response and saves results
fn process_ai_response(
    module: &mut BusinessProcessAnalysisModule,
    request: &mut Individual,
    response: crate::ai_client::AIResponseValues,
    schema: &mut ResponseSchema,
) -> Result<(), Box<dyn std::error::Error>> {
    // Convert HashMap to Value
    let response_value = response.to_json_value();
    info!("Parsing AI response into structured format...");
    let mut parse_result = schema.parse_ai_response(&response_value, module)?;

    // Save and link results
    let result_id = parse_result.main_individual.get_id().to_string();
    info!("Saving main result individual ID: {}", result_id);

    if let Err(e) = module.backend.mstorage_api.update_or_err(&module.ticket, "", "BPA", IndvOp::Put, &mut parse_result.main_individual) {
        error!("Failed to save main individual {}: {:?}", result_id, e);
        return Err(format!("Failed to save main individual {}: {:?}", result_id, e).into());
    }

    // Save related individuals
    let related_count = parse_result.related_individuals.len();
    info!("Saving {} related individuals...", related_count);

    for (index, mut related) in parse_result.related_individuals.into_iter().enumerate() {
        info!("Saving related individual {}/{}: {}", index + 1, related_count, related.get_id());
        if let Err(e) = module.backend.mstorage_api.update_or_err(&module.ticket, "", "BPA", IndvOp::Put, &mut related) {
            error!("Failed to save related individual {}: {:?}", related.get_id(), e);
            return Err(format!("Failed to save related individual {}: {:?}", related.get_id(), e).into());
        }
    }

    // Update request
    info!("Updating request {} with result reference {}", request.get_id(), result_id);
    request.set_uri("v-bpa:hasResult", &result_id);
    request.set_uri("v-bpa:processingStatus", "v-bpa:Completed");

    if let Err(e) = module.backend.mstorage_api.update_or_err(&module.ticket, "", "BPA", IndvOp::Put, request) {
        error!("Failed to update request {}: {:?}", request.get_id(), e);
        return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("Failed to update request {}: {:?}", request.get_id(), e))));
    }

    // Trigger parent processing
    info!("Triggering parent processing for request {}", request.get_id());
    if let Err(e) = trigger_parent_processing(module, request) {
        warn!("Failed to trigger parent processing for request {}: {:?}", request.get_id(), e);
    }

    info!("Successfully processed AI response for request {}", request.get_id());
    Ok(())
}

/// Triggers parent object processing by updating v-s:modified
fn trigger_parent_processing(module: &mut BusinessProcessAnalysisModule, request: &mut Individual) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent_id) = request.get_first_literal("v-s:hasParentLink") {
        info!("Triggering processing for parent object: {}", parent_id);

        let mut update = Individual::default();
        update.set_id(&parent_id);
        update.set_datetime("v-s:modified", Utc::now().timestamp());

        if let Err(e) = module.backend.mstorage_api.update_or_err(&module.ticket, "", "trigger", IndvOp::SetIn, &mut update) {
            error!("Failed to trigger parent processing for {}: {:?}", parent_id, e);
            return Err(format!("Failed to trigger processing for {}: {:?}", parent_id, e).into());
        }

        info!("Successfully triggered processing for parent {}", parent_id);
    } else {
        info!("No parent link found for request {}", request.get_id());
    }
    Ok(())
}
