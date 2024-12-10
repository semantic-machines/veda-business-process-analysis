use crate::document_status_handler::reset_document_status;
use crate::extractors::extract_text_from_document;
use crate::queue_processor::BusinessProcessAnalysisModule;
use chrono::Utc;
use std::fs;
use std::path::Path;
use uuid::Uuid;
use v_common::onto::datatype::Lang;
use v_common::onto::individual::Individual;
use v_common::v_api::api_client::IndvOp;
use v_common::v_api::obj::ResultCode;

// Stage weights for progress calculation
const FILE_PROCESSING_WEIGHT: f32 = 0.3; // 30% for initial file processing
const TEXT_EXTRACTION_WEIGHT: f32 = 0.3; // 30% for text extraction
const DOCUMENT_ANALYSIS_WEIGHT: f32 = 0.4; // 40% for document analysis

/// Gets progress from child process
fn get_child_progress(module: &mut BusinessProcessAnalysisModule, child_id: &str) -> f32 {
    if let Some(mut child) = module.backend.get_individual_s(child_id) {
        if let Some(progress) = child.get_first_integer("v-bpa:percentComplete") {
            return progress as f32 / 100.0;
        }
    }
    0.0
}

/// Updates pipeline progress including child process progress
fn update_pipeline_progress(
    module: &mut BusinessProcessAnalysisModule,
    pipeline: &mut Individual,
    current_stage: &str,
    stage_progress: f32,
    child_id: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Get child progress if available
    let child_progress = if let Some(id) = child_id {
        get_child_progress(module, id)
    } else {
        stage_progress
    };

    // Calculate total progress based on stage weights and current progress
    let total_progress = match current_stage {
        "initial_processing" => child_progress * FILE_PROCESSING_WEIGHT,
        "text_extraction" => FILE_PROCESSING_WEIGHT + (child_progress * TEXT_EXTRACTION_WEIGHT),
        "document_analysis" => FILE_PROCESSING_WEIGHT + TEXT_EXTRACTION_WEIGHT + (child_progress * DOCUMENT_ANALYSIS_WEIGHT),
        _ => 0.0,
    };

    // Convert to percentage (0-100) and round to integer
    let progress_percent = (total_progress * 100.0).round() as i64;

    // Update pipeline progress
    pipeline.set_integer("v-bpa:percentComplete", progress_percent);

    // Save updated progress
    if let Err(e) = module.backend.mstorage_api.update_or_err(&module.ticket, "", "BPA", IndvOp::SetIn, pipeline) {
        error!("Failed to update pipeline progress: {:?}", e);
        return Err(format!("Failed to update pipeline progress: {:?}", e).into());
    }

    Ok(())
}

/// Reads file content from attachment
fn read_attachment_content(module: &mut BusinessProcessAnalysisModule, attachment_id: &str) -> Result<(Vec<u8>, String), Box<dyn std::error::Error>> {
    // Load attachment individual
    let mut attachment = Individual::default();
    if module.backend.storage.get_individual(attachment_id, &mut attachment) != ResultCode::Ok {
        error!("Failed to load attachment {}", attachment_id);
        return Err(format!("Failed to load attachment {}", attachment_id).into());
    }

    // Get file extension
    let filename = attachment.get_first_literal("v-s:fileName").ok_or("No filename in attachment")?;
    let extension = filename.split('.').last().ok_or("Invalid filename format")?.to_lowercase();

    // Get file path
    let file_uri = attachment.get_first_literal("v-s:fileUri").ok_or("No file URI in attachment")?;
    let file_path = attachment.get_first_literal("v-s:filePath").ok_or("No file path in attachment")?;
    let full_path = format!("./data/files/{}/{}", file_path, file_uri);

    if !Path::new(&full_path).exists() {
        error!("File does not exist: {}", full_path);
        return Err("File not found".into());
    }

    let content = fs::read(&full_path).map_err(|e| {
        error!("Failed to read file: {}", e);
        format!("Failed to read file {}: {}", full_path, e)
    })?;

    Ok((content, extension))
}

/// Creates a new processing request with progress tracking
fn create_processing_request(
    module: &mut BusinessProcessAnalysisModule,
    pipeline: &mut Individual,
    prompt_id: &str,
    raw_input: String,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut request = Individual::default();
    let request_id = format!("d:request_{}", Uuid::new_v4());
    request.set_id(&request_id);
    request.set_uri("rdf:type", "v-bpa:GenericProcessingRequest");
    request.set_uri("v-bpa:prompt", prompt_id);
    request.set_string("v-bpa:rawInput", &raw_input, Lang::none());
    request.set_uri("v-s:hasParentLink", pipeline.get_id());

    if let Err(e) = module.backend.mstorage_api.update_or_err(&module.ticket, pipeline.get_id(), "PIPELINE", IndvOp::Put, &mut request) {
        error!("Failed to create processing request: {:?}", e);
        return Err(format!("Failed to create request: {:?}", e).into());
    }

    Ok(request_id)
}

pub fn raw_document_extracting_and_structuring(module: &mut BusinessProcessAnalysisModule, pipeline: &mut Individual) -> Result<(), Box<dyn std::error::Error>> {
    info!("=== Starting Document Processing Pipeline ===");
    info!("Pipeline request ID: {}", pipeline.get_id());

    // Get current stage
    let current_stage = pipeline.get_first_literal("v-bpa:currentStage").unwrap_or_default();

    match current_stage.as_str() {
        "" => {
            // Initial stage - create text extraction request
            info!("Starting document processing...");
            // Get attachment
            let attachment_id = pipeline.get_first_literal("v-s:attachment").ok_or("No attachment found in pipeline")?;

            // Read file content
            let (content, extension) = read_attachment_content(module, &attachment_id)?;
            info!("Successfully read file content, extension: {}", extension);

            // Extract text from document
            let extracted_contents = extract_text_from_document(&content, &extension)?;
            info!("Successfully extracted text from document");

            // Create intermediate result with extracted text
            let mut text_result = Individual::default();
            let result_id = format!("d:text_result_{}", Uuid::new_v4());
            text_result.set_id(&result_id);
            text_result.set_uri("rdf:type", "v-bpa:ExtractedTextResult");

            // Join all extracted text parts
            let combined_text = extracted_contents.join("\n\n");
            text_result.set_string("v-bpa:extractedText", &combined_text, Lang::none());

            if let Err(e) = module.backend.mstorage_api.update_or_err(&module.ticket, "", "BPA", IndvOp::Put, &mut text_result) {
                error!("Failed to save extracted text result: {:?}", e);
                return Err(format!("Failed to save text result: {:?}", e).into());
            }

            // Create document analysis request
            let request_id = create_processing_request(module, pipeline, "v-bpa:DocumentAnalysisPrompt", combined_text)?;

            // Update pipeline status
            pipeline.set_string("v-bpa:currentStage", "document_analysis", Lang::none());
            pipeline.set_uri("v-bpa:hasExecutionState", "v-bpa:ExecutionInProgress");
            pipeline.set_uri("v-bpa:hasNextStage", &request_id);
            pipeline.set_datetime("v-bpa:startDate", Utc::now().timestamp());

            update_pipeline_progress(module, pipeline, "document_analysis", 0.0, None)?;

            if let Err(e) = module.backend.mstorage_api.update_or_err(&module.ticket, "", "BPA", IndvOp::SetIn, pipeline) {
                error!("Failed to update pipeline status: {:?}", e);
                return Err(format!("Failed to update pipeline: {:?}", e).into());
            }
        },
        "document_analysis" => {
            // Check if document analysis completed
            if let Some(next_stage_id) = pipeline.get_first_literal("v-bpa:hasNextStage") {
                let mut next_stage = Individual::default();
                if module.backend.storage.get_individual(&next_stage_id, &mut next_stage) != ResultCode::Ok {
                    return Err(format!("Failed to load next stage: {}", next_stage_id).into());
                }

                // Update progress based on analysis status and child progress
                update_pipeline_progress(module, pipeline, "document_analysis", 0.0, Some(&next_stage_id))?;

                if next_stage.any_exists("v-bpa:processingStatus", &["v-bpa:Completed"]) {
                    info!("Document analysis completed, finalizing pipeline...");

                    let result_id = next_stage.get_first_literal("v-bpa:hasResult").ok_or("No result found")?;

                    // Update pipeline status
                    pipeline.set_uri("v-bpa:resultDocument", &result_id);
                    pipeline.set_uri("v-bpa:hasExecutionState", "v-bpa:ExecutionCompleted");
                    pipeline.set_uri("v-bpa:processingStatus", "v-bpa:Completed");
                    pipeline.set_datetime("v-bpa:endDate", Utc::now().timestamp());
                    pipeline.remove("v-bpa:currentStage");
                    pipeline.remove("v-bpa:hasNextStage");

                    if let Err(e) = module.backend.mstorage_api.update_or_err(&module.ticket, "", "BPA", IndvOp::SetIn, pipeline) {
                        error!("Failed to update pipeline status: {:?}", e);
                        return Err(format!("Failed to update pipeline: {:?}", e).into());
                    }

                    // Reset status tags as document was processed
                    if let Err(e) = reset_document_status(module, &result_id) {
                        warn!("Failed to reset document status: {:?}", e);
                    }

                    info!("Pipeline completed successfully");
                }
            }
        },
        _ => {
            error!("Unknown pipeline stage: {}", current_stage);
            return Err(format!("Invalid pipeline stage: {}", current_stage).into());
        },
    }

    Ok(())
}
