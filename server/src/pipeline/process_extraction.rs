use crate::ai_client::send_text_request_to_ai;
use crate::common::{get_prompt_text, ClientType};
use crate::generic_processing_handler::process_generic_request;
use crate::queue_processor::BusinessProcessAnalysisModule;
use chrono::Utc;
use openai_dive::v1::resources::chat::{ChatCompletionParametersBuilder, ChatMessage, ChatMessageContent};
use serde_json::json;
use tokio::runtime::Runtime;
use v_common::onto::datatype::Lang;
use v_common::onto::individual::Individual;
use v_common::v_api::api_client::IndvOp;
use v_common::v_api::obj::ResultCode;

/// Process extraction pipeline handler
pub fn process_extraction_pipeline(module: &mut BusinessProcessAnalysisModule, pipeline: &mut Individual) -> Result<(), Box<dyn std::error::Error>> {
    info!("=== Starting Process Extraction Pipeline ===");
    info!("Pipeline ID: {}", pipeline.get_id());

    // Update start time and state
    info!("Initializing pipeline state...");
    let start_time = Utc::now().timestamp();
    pipeline.set_uri("v-bpa:hasExecutionState", "v-bpa:ExecutionInProgress");
    pipeline.set_datetime("v-bpa:startDate", start_time);
    pipeline.set_uri("v-bpa:processingStatus", "v-bpa:Processing");

    // Save initial state
    info!("Saving initial pipeline state to database...");
    if let Err(e) = module.backend.mstorage_api.update_or_err(&module.ticket, "", "BPA", IndvOp::SetIn, pipeline) {
        error!("Failed to save initial state: {:?}", e);
        return Err(format!("Failed to update pipeline state: {:?}", e).into());
    }

    // Get prompt text
    let prompt_text = get_prompt_text(module, "v-bpa:ProcessExtractionPrompt")?;
    info!("Retrieved extraction prompt");

    // Get target department
    let department = pipeline.get_first_literal("v-bpa:targetDepartment").ok_or_else(|| {
        error!("No target department found in pipeline configuration");
        "No target department specified"
    })?;
    info!("Target Department: {}", department);

    // Get required section types
    let section_types = pipeline.get_literals("v-bpa:hasDocumentSectionTypes").ok_or_else(|| {
        error!("No document section types found in pipeline configuration");
        "No document section types specified"
    })?;
    info!("Looking for sections of types: {:?}", section_types);

    debug!("Building search query for documents...");
    let query = format!("'rdf:type' == 'v-bpa:ProcessDocument' && 'v-bpa:hasDepartment' == '{}'", department);
    debug!("Search query: {}", query);

    // Find all relevant documents
    info!("Executing search for documents...");
    let document_ids = module.xr.query(v_common::search::common::FTQuery::new_with_user("cfg:VedaSystem", &query), &mut module.backend.storage).result;

    info!("Found {} documents to process", document_ids.len());
    if document_ids.is_empty() {
        warn!("No documents found for department {}", department);
    }

    let mut documents_data = Vec::new();
    let mut processed_docs = 0;
    let mut skipped_docs = 0;
    let mut total_processed_sections = 0;

    // Set initial estimated time
    let initial_estimated_time = calculate_estimated_time(document_ids.len(), processed_docs, 0);
    pipeline.set_integer("v-bpa:estimatedTime", initial_estimated_time);
    if let Err(e) = module.backend.mstorage_api.update_or_err(&module.ticket, "", "BPA", IndvOp::SetIn, pipeline) {
        warn!("Failed to update initial estimated time: {:?}", e);
    }

    info!("=== Starting Document Processing ===");
    for (doc_index, doc_id) in document_ids.iter().enumerate() {
        info!("Processing document {}/{}: {}", doc_index + 1, document_ids.len(), doc_id);

        let mut document = Individual::default();
        if module.backend.storage.get_individual(doc_id, &mut document) != ResultCode::Ok {
            warn!("Failed to load document {}", doc_id);
            skipped_docs += 1;
            continue;
        }
        document.parse_all();

        let mut doc_sections = Vec::new();

        // Process document sections
        if let Some(sections) = document.get_literals("v-bpa:hasDocumentSection") {
            info!("Document has {} sections", sections.len());

            for (section_index, section) in sections.iter().enumerate() {
                info!("Processing section {}/{}", section_index + 1, sections.len());

                let mut section_indv = Individual::default();
                if module.backend.storage.get_individual(section, &mut section_indv) != ResultCode::Ok {
                    warn!("Failed to load section {}", section);
                    continue;
                }
                section_indv.parse_all();

                if let Some(section_type) = section_indv.get_first_literal("v-bpa:hasSectionType") {
                    info!("Section type: {}", section_type);
                    if section_types.contains(&section_type) {
                        info!("Found matching section type: {}", section_type);

                        let title = section_indv.get_first_literal("v-bpa:sectionTitle").unwrap_or_default();
                        let content = section_indv.get_first_literal("v-bpa:sectionContent").unwrap_or_default();
                        debug!("Section title: {}", title);
                        debug!("Content length: {} characters", content.len());

                        let section_json = json!({
                            "section_type": section_type,
                            "title": title,
                            "content": content
                        });
                        doc_sections.push(section_json);
                        total_processed_sections += 1;
                    } else {
                        debug!("Skipping section with non-matching type: {}", section_type);
                    }
                }
            }
        }

        // Add document to results if it has matching sections
        if !doc_sections.is_empty() {
            debug!("Adding document with {} matching sections to results", doc_sections.len());

            let doc_json = json!({
                "department": document.get_first_literal("v-bpa:documentDepartment").unwrap_or_default(),
                "documentTitle": document.get_first_literal("v-bpa:documentTitle").unwrap_or_default(),
                "documentType": document.get_first_literal("v-bpa:documentType").unwrap_or_default(),
                "documentSource": document.get_first_literal("v-bpa:documentSource").unwrap_or_default(),
                "documentSignedDate": document.get_first_literal("v-bpa:documentSignedDate").unwrap_or_default(),
                "documentSignedBy": document.get_first_literal("v-bpa:documentSignedBy").unwrap_or_default(),
                "sections": doc_sections
            });
            documents_data.push(doc_json);
            processed_docs += 1;
        } else {
            debug!("Document has no matching sections, skipping");
            skipped_docs += 1;
        }

        // Update estimated time after each document
        let current_time = Utc::now().timestamp();
        let elapsed_time = current_time - start_time;
        let estimated_time = calculate_estimated_time(document_ids.len(), processed_docs, elapsed_time);

        pipeline.set_integer("v-bpa:estimatedTime", estimated_time);
        if let Err(e) = module.backend.mstorage_api.update_or_err(&module.ticket, "", "BPA", IndvOp::SetIn, pipeline) {
            warn!("Failed to update pipeline estimated time: {:?}", e);
        }
        debug!("Updated estimated time: {} seconds", estimated_time);
    }

    info!("=== Document Processing Statistics ===");
    info!("Total documents found: {}", document_ids.len());
    info!("Documents with matching sections: {}", processed_docs);
    info!("Documents without matching sections: {}", skipped_docs);
    info!("Total processed sections: {}", total_processed_sections);

    // Create input data with prompt and documents
    let input_json = json!({
        "prompt": prompt_text,
        "documents": documents_data
    });

    let input_json_string = serde_json::to_string_pretty(&input_json)?;

    // Prepare parameters for reasoning model
    let parameters = ChatCompletionParametersBuilder::default()
        .model(module.reasoning_model.clone())
        .seed(43u32)
        .messages(vec![ChatMessage::User {
            content: ChatMessageContent::Text(input_json_string),
            name: None,
        }])
        .build()?;

    // Send request to reasoning model
    info!("Sending request to reasoning model...");
    let rt = Runtime::new()?;
    let ai_response = rt.block_on(async { send_text_request_to_ai(module, parameters, ClientType::Reasoning).await })?;

    // Extract text from response and save it
    let response_text = ai_response.get("result").and_then(|v| v.as_str()).ok_or("Failed to get text from AI response")?;

    // Process extracted text through ProcessListExtractionPrompt
    create_and_process_extraction_request(module, response_text, pipeline)?;

    // Update pipeline status
    info!("Updating pipeline completion status...");
    pipeline.set_uri("v-bpa:hasExecutionState", "v-bpa:ExecutionCompleted");
    pipeline.set_uri("v-bpa:processingStatus", "v-bpa:Completed");
    pipeline.set_datetime("v-bpa:endDate", Utc::now().timestamp());

    if let Err(e) = module.backend.mstorage_api.update_or_err(&module.ticket, "", "BPA", IndvOp::SetIn, pipeline) {
        return Err(format!("Failed to update pipeline completion state: {:?}", e).into());
    }

    info!("=== Process Extraction Pipeline Completed Successfully ===");
    Ok(())
}

/// Calculate estimated time based on remaining documents and average processing time
fn calculate_estimated_time(total_docs: usize, processed_docs: usize, elapsed_time: i64) -> i64 {
    if processed_docs == 0 {
        // Initial estimate based on assumption of 1 minute per document
        return (total_docs as i64) * 60;
    }

    // Calculate average time per document
    let avg_time_per_doc = elapsed_time as f64 / processed_docs as f64;
    let remaining_docs = total_docs - processed_docs;

    // Calculate estimated remaining time
    (avg_time_per_doc * remaining_docs as f64) as i64
}

/// Handles errors in pipeline execution
pub(crate) fn handle_pipeline_error(module: &mut BusinessProcessAnalysisModule, pipeline: &mut Individual, error: Box<dyn std::error::Error>) {
    error!("=== Pipeline Execution Failed ===");
    error!("Error details: {}", error);

    info!("Updating pipeline error state...");
    pipeline.set_uri("v-bpa:hasExecutionState", "v-bpa:ExecutionError");
    pipeline.set_uri("v-bpa:processingStatus", "v-bpa:Failed");
    pipeline.set_string("v-bpa:lastError", &error.to_string(), Lang::none());
    pipeline.set_datetime("v-bpa:endDate", Utc::now().timestamp());

    if let Err(e) = module.backend.mstorage_api.update_or_err(&module.ticket, "", "BPA", IndvOp::SetIn, pipeline) {
        error!("Failed to update pipeline error state: {:?}", e);
        error!("Additional error occurred while handling original error");
    }
    error!("=== Pipeline Error Handling Completed ===");
}

/// Creates and processes a request to extract business processes from text
/// Creates and processes a request to extract business processes from text
fn create_and_process_extraction_request(
    module: &mut BusinessProcessAnalysisModule,
    response_text: &str,
    pipeline: &mut Individual,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Creating process extraction request...");

    // Create request individual
    let mut request = Individual::default();
    let request_id = format!("d:request_{}", uuid::Uuid::new_v4());
    request.set_id(&request_id);
    request.set_uri("rdf:type", "v-bpa:GenericProcessingRequest");
    request.set_uri("v-bpa:prompt", "v-bpa:ProcessListExtractionPrompt");
    request.set_string("v-bpa:rawInput", response_text, Lang::none());
    request.set_string("v-bpa:targetType", "v-bpa:BusinessProcess", Lang::none());
    request.set_uri("v-bpa:processingStatus", "v-bpa:Processing");

    // Add link to pipeline source
    request.set_uri("v-s:hasParentLink", pipeline.get_id());

    // Save request to storage
    if let Err(e) = module.backend.mstorage_api.update_or_err(&module.ticket, "", "BPA", IndvOp::Put, &mut request) {
        error!("Failed to create extraction request: {:?}", e);
        return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("Failed to create request, err={:?}", e))));
    }

    info!("Created process extraction request: {}", request_id);

    // Process the request using generic handler
    if let Err(e) = process_generic_request(module, &mut request) {
        error!("Failed to process extraction request: {:?}", e);
        return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("Failed to process request, err={:?}", e))));
    }

    info!("Successfully processed extraction request: {}", request_id);

    // Link pipeline to next stage instead of result
    pipeline.add_uri("v-bpa:hasNextStage", &request_id);

    Ok(())
}
