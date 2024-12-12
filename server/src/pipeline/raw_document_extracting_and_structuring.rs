use crate::common::generate_event_id;
use crate::document_status_handler::reset_document_status;
use crate::extractors::types::ExtractedContent;
use crate::extractors::types::ExtractedContent::Text;
use crate::extractors::{extract_count_pages_document, extract_texts_or_images_from_document};
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
const FILE_PROCESSING_WEIGHT: f32 = 0.01; // 1% for initial file processing
const DOCUMENT_ANALYSIS_WEIGHT: f32 = 0.42;

fn calculate_text_extraction_weight(total_pages: usize) -> f32 {
    let remaining_weight = 1.0 - (FILE_PROCESSING_WEIGHT + DOCUMENT_ANALYSIS_WEIGHT);
    if total_pages == 0 {
        return 0.1;
    }
    remaining_weight
}

fn calculate_estimated_time(pipeline: &mut Individual, current_stage: &str, stage_progress: f32, total_pages: Option<usize>) -> i64 {
    let current_time = Utc::now().timestamp();

    // Get pipeline start time
    let pipeline_start = pipeline.get_first_datetime("v-bpa:startDate").unwrap_or_else(|| {
        let now = Utc::now().timestamp();
        pipeline.set_datetime("v-bpa:startDate", now);
        now
    });

    let total_elapsed = current_time - pipeline_start;
    let base_request_time = 27;

    let estimated = match current_stage {
        "initial_processing" => {
            if stage_progress >= 1.0 {
                // Estimate for remaining stages based on page count
                let pages = total_pages.unwrap_or(1) as i64;
                (pages * base_request_time) + 60 // Add time for analysis stage
            } else {
                // Initial estimate based on page count
                total_pages.unwrap_or(1) as i64 * base_request_time
            }
        },
        "text_extraction" => {
            if stage_progress >= 1.0 {
                // Estimate for final analysis stage
                60 // ~1 minute for analysis
            } else {
                // Calculate based on elapsed time
                let time_per_percent = total_elapsed as f32 / (stage_progress * 100.0);
                let remaining_percent = (1.0 - stage_progress) * 100.0;
                (time_per_percent * remaining_percent) as i64 + 60 // Add time for final stage
            }
        },
        "document_analysis" => {
            if stage_progress >= 1.0 {
                0 // Complete
            } else if stage_progress <= 0.0 {
                // Initial estimate for analysis
                45 // 45 seconds for analysis
            } else {
                // Calculate remaining time
                let time_per_percent = total_elapsed as f32 / (stage_progress * 100.0);
                let remaining_percent = (1.0 - stage_progress) * 100.0;
                (time_per_percent * remaining_percent) as i64
            }
        },
        _ => 0,
    };

    estimated.max(0) // Ensure we never return negative time
}

fn update_pipeline_progress(
    event_id: &str,
    module: &mut BusinessProcessAnalysisModule,
    pipeline: &mut Individual,
    current_stage: &str,
    stage_progress: f32,
    total_pages: Option<usize>,
) -> Result<(), Box<dyn std::error::Error>> {
    let text_extraction_weight = match total_pages {
        Some(pages) => calculate_text_extraction_weight(pages),
        None => 0.66,
    };

    let total_progress = match current_stage {
        "initial_processing" => FILE_PROCESSING_WEIGHT,
        "text_extraction" => FILE_PROCESSING_WEIGHT + (stage_progress * text_extraction_weight),
        "document_analysis" => {
            let doc_analysis_progress = if stage_progress > 0.0 {
                stage_progress
            } else {
                0.0
            };
            FILE_PROCESSING_WEIGHT + text_extraction_weight + (doc_analysis_progress * DOCUMENT_ANALYSIS_WEIGHT)
        },
        _ => 0.0,
    };

    let progress_percent = (total_progress * 100.0).round() as i64;

    // Calculate estimated remaining time
    let estimated_time = calculate_estimated_time(pipeline, current_stage, stage_progress, total_pages);

    info!(
        "Pipeline [{}]: stage={}, pages={:?}, text_extraction_weight={:.2}, stage_progress={:.2}, total_progress={:.2}, percent={}%, estimated_time={}s",
        pipeline.get_id(),
        current_stage,
        total_pages,
        text_extraction_weight,
        stage_progress,
        total_progress,
        progress_percent,
        estimated_time
    );

    pipeline.set_integer("v-bpa:percentComplete", progress_percent);
    pipeline.set_integer("v-bpa:estimatedTime", estimated_time);

    if let Err(e) = module.backend.mstorage_api.update_or_err(&module.ticket, event_id, "BPA", IndvOp::SetIn, pipeline) {
        error!("Pipeline [{}]: failed to update progress to {}%: {:?}", pipeline.get_id(), progress_percent, e);
        return Err(format!("Failed to update pipeline progress: {:?}", e).into());
    }

    Ok(())
}

/// Reads file content from attachment
fn read_attachment_content(module: &mut BusinessProcessAnalysisModule, attachment_id: &str) -> Result<(Vec<u8>, String), Box<dyn std::error::Error>> {
    // Load attachment individual
    let mut attachment = Individual::default();
    if module.backend.storage.get_individual(attachment_id, &mut attachment) != ResultCode::Ok {
        error!("Failed to load attachment [{}]", attachment_id);
        return Err(format!("Failed to load attachment [{}]", attachment_id).into());
    }

    // Get file extension
    let filename = attachment.get_first_literal("v-s:fileName").ok_or("No filename in attachment")?;
    let extension = filename.split('.').last().ok_or("Invalid filename format")?.to_lowercase();

    // Get file path
    let file_uri = attachment.get_first_literal("v-s:fileUri").ok_or("No file URI in attachment")?;
    let file_path = attachment.get_first_literal("v-s:filePath").ok_or("No file path in attachment")?;
    let full_path = format!("./data/files/{}/{}", file_path, file_uri);

    if !Path::new(&full_path).exists() {
        error!("Attachment [{}]: file does not exist at path [{}]", attachment_id, full_path);
        return Err(format!("File not found: [{}]", full_path).into());
    }

    let content = fs::read(&full_path).map_err(|e| {
        error!("Attachment [{}]: failed to read file [{}]: {}", attachment_id, full_path, e);
        format!("Failed to read file [{}]: {}", full_path, e)
    })?;

    info!("Attachment [{}]: successfully read file [{}], size={} bytes", attachment_id, filename, content.len());
    Ok((content, extension))
}

/// Creates a new processing request with progress tracking
fn create_processing_request(
    event_id: &str,
    module: &mut BusinessProcessAnalysisModule,
    pipeline: &mut Individual,
    prompt_id: &str,
    content: ExtractedContent,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut request = Individual::default();
    let request_id = format!("d:request_{}", Uuid::new_v4());
    request.set_id(&request_id);
    request.set_uri("rdf:type", "v-bpa:GenericProcessingRequest");
    request.set_uri("v-bpa:prompt", prompt_id);

    match content {
        ExtractedContent::Text(t) => {
            request.set_string("v-bpa:rawInput", &t, Lang::none());
        },
        ExtractedContent::ImageFile {
            path,
            name,
            ..
        } => {
            let mut attachment = Individual::default();
            let attachment_id = format!("d:attachment_{}", Uuid::new_v4());
            attachment.set_id(&attachment_id);
            attachment.set_uri("rdf:type", "v-s:File");
            attachment.set_string("v-s:filePath", &path, Lang::none());
            attachment.set_string("v-s:fileUri", &name, Lang::none());

            if let Err(e) = module.backend.mstorage_api.update_or_err(&module.ticket, event_id, "PIPELINE", IndvOp::Put, &mut attachment) {
                error!("Pipeline [{}]: failed to create request [{}] with prompt [{}]: {:?}", pipeline.get_id(), request_id, prompt_id, e);
                return Err(format!("Failed to create attachment: {:?}", e).into());
            }

            request.add_uri("v-s:attachment", &attachment_id);
        },
    }

    request.set_uri("v-s:hasParentLink", pipeline.get_id());

    if let Err(e) = module.backend.mstorage_api.update_or_err(&module.ticket, event_id, "PIPELINE", IndvOp::Put, &mut request) {
        error!("Pipeline [{}]: failed to create request [{}] with prompt [{}]: {:?}", pipeline.get_id(), request_id, prompt_id, e);
        return Err(format!("Failed to create request: {:?}", e).into());
    }

    info!("Pipeline [{}]: created request [{}] with prompt [{}]", pipeline.get_id(), request_id, prompt_id);
    Ok(request_id)
}

pub fn raw_document_extracting_and_structuring(
    module: &mut BusinessProcessAnalysisModule,
    pipeline_in_queue: &mut Individual,
    in_event_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let event_id = match generate_event_id("RDEASP", pipeline_in_queue.get_id(), in_event_id) {
        Some(s) => s,
        None => return Ok(()),
    };

    if let Err(e) = raw_document_extracting_and_structuring_internal(module, pipeline_in_queue, &event_id) {
        error!("Processing failed: {:?}", e);

        // Set error status and details
        pipeline_in_queue.set_uri("v-bpa:hasExecutionState", "v-bpa:ExecutionError");
        pipeline_in_queue.set_string("v-bpa:lastError", &e.to_string(), Lang::none());

        // Save error status
        if let Err(update_err) = module.backend.mstorage_api.update_or_err(&module.ticket, &event_id, "BPA", IndvOp::SetIn, pipeline_in_queue) {
            error!("Failed to update pipeline error status: {:?}", update_err);
            return Err(format!("Failed to update pipeline: {:?}", update_err).into());
        }

        return Err(e);
    }

    Ok(())
}

fn raw_document_extracting_and_structuring_internal(
    module: &mut BusinessProcessAnalysisModule,
    pipeline_in_queue: &mut Individual,
    event_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("=== Pipeline [{}] ===", pipeline_in_queue.get_id());

    // reread current state piprline
    let mut pipeline = Individual::default();
    if module.backend.storage.get_individual(&pipeline_in_queue.get_id(), &mut pipeline) != ResultCode::Ok {
        error!("Pipeline [{}]: failed to load", pipeline.get_id());
        return Err(format!("Failed to load pipeline [{}]", pipeline.get_id()).into());
    }

    // Get current stage
    let current_stage = pipeline.get_first_literal("v-bpa:currentStage").unwrap_or_default();

    match current_stage.as_str() {
        "" => {
            let attachment_id = pipeline.get_first_literal("v-s:attachment").ok_or("No attachment found in pipeline")?;
            let mut attachment = Individual::default();
            if module.backend.storage.get_individual(&attachment_id, &mut attachment) != ResultCode::Ok {
                error!("Pipeline [{}]: failed to load attachment [{}]", pipeline.get_id(), attachment_id);
                return Err(format!("Failed to load attachment [{}]", attachment_id).into());
            }

            let filename = attachment.get_first_literal("v-s:fileName").unwrap_or_default();
            info!("Pipeline [{}]: start processing document [{}] ({})", pipeline.get_id(), filename, attachment_id);

            // Read file content
            let (content, extension) = read_attachment_content(module, &attachment_id)?;

            // Extract text or images from document
            let total_pages = extract_count_pages_document(&content, &extension)?;
            update_pipeline_progress(event_id, module, &mut pipeline, "initial_processing", 1.0, Some(total_pages as usize))?;

            let extracted_contents = extract_texts_or_images_from_document(&content, &extension)?;
            info!("Pipeline [{}]: extracted {} content parts from document [{}]", pipeline.get_id(), extracted_contents.len(), attachment_id);

            // Create recognition requests for each content part
            for (idx, content) in extracted_contents.iter().enumerate() {
                let request_id = create_processing_request(event_id, module, &mut pipeline, "v-bpa:ImagesToTextPrompt", content.clone())?;
                info!("Pipeline [{}]: created recognition request [{}] for part {}/{}", pipeline.get_id(), request_id, idx + 1, extracted_contents.len());
                pipeline.add_uri("v-bpa:hasStageRequest", &request_id);

                if let Err(e) = module.backend.mstorage_api.update_or_err(&module.ticket, &format!("WKUP_{}", event_id), "", IndvOp::SetIn, &pipeline) {
                    error!("Pipeline [{}]: failed to update hasStageRequest: {:?}", pipeline.get_id(), e);
                    return Err(format!("Failed to update pipeline: {:?}", e).into());
                }
            }

            // Update pipeline stage
            pipeline.set_string("v-bpa:currentStage", "content_recognize", Lang::none());
            pipeline.set_uri("v-bpa:hasExecutionState", "v-bpa:ExecutionInProgress");
            pipeline.set_datetime("v-bpa:startDate", Utc::now().timestamp());

            if let Err(e) = module.backend.mstorage_api.update_or_err(&module.ticket, event_id, "BPA", IndvOp::SetIn, &pipeline) {
                error!("Pipeline [{}]: failed to update status: {:?}", pipeline.get_id(), e);
                return Err(format!("Failed to update pipeline: {:?}", e).into());
            }

            info!("Pipeline [{}]: initialized", pipeline.get_id());
            update_pipeline_progress(event_id, module, &mut pipeline, "initial_processing", 1.0, Some(extracted_contents.len()))?;
        },
        "content_recognize" => {
            let request_ids = pipeline.get_literals("v-bpa:hasStageRequest").unwrap_or_default();

            let mut all_completed = true;
            let mut combined_text = String::new();
            let mut completed_count = 0;
            let total_request_count = request_ids.len();

            for request_id in &request_ids {
                let mut request = Individual::default();
                if module.backend.storage.get_individual(request_id, &mut request) != ResultCode::Ok {
                    error!("Pipeline [{}]: failed to load request [{}]", pipeline.get_id(), request_id);
                    return Err(format!("Failed to load request [{}]", request_id).into());
                }

                if !request.any_exists("v-bpa:processingStatus", &["v-bpa:Completed"]) {
                    // Check for failed status
                    if request.any_exists("v-bpa:processingStatus", &["v-bpa:Failed"]) {
                        if let Some(error) = request.get_first_literal("v-bpa:lastError") {
                            error!("Pipeline [{}]: request [{}] failed with error: {}", pipeline.get_id(), request_id, error);
                            return Err(format!("Content recognition failed: {}", error).into());
                        } else {
                            error!("Pipeline [{}]: request [{}] failed without error details", pipeline.get_id(), request_id);
                            return Err("Content recognition failed without details".into());
                        }
                    }
                    all_completed = false;
                    continue;
                }

                completed_count += 1;

                if let Some(result_id) = request.get_first_literal("v-bpa:hasResult") {
                    let mut result = Individual::default();
                    if module.backend.storage.get_individual(&result_id, &mut result) == ResultCode::Ok {
                        if let Some(text) = result.get_first_literal("v-bpa:extractedText") {
                            info!("Pipeline [{}]: got extracted text from result [{}], length={}", pipeline.get_id(), result_id, text.len());
                            combined_text.push_str(&text);
                            combined_text.push('\n');
                        }
                    }
                }
            }

            let stage_progress = if total_request_count == 0 {
                0.0
            } else {
                completed_count as f32 / total_request_count as f32
            };

            update_pipeline_progress(event_id, module, &mut pipeline, "text_extraction", stage_progress, Some(total_request_count))?;

            if !all_completed {
                info!("Pipeline [{}]: content recognition in progress, {}/{} requests completed", pipeline.get_id(), completed_count, total_request_count);
                return Ok(());
            }

            info!("Pipeline [{}]: all {} recognition requests completed", pipeline.get_id(), request_ids.len());

            pipeline.remove("v-bpa:hasStageRequest");
            let request_id = create_processing_request(
                event_id,
                module,
                &mut pipeline,
                "v-bpa:DocumentAnalysisPrompt",
                Text {
                    0: combined_text,
                },
            )?;
            pipeline.add_uri("v-bpa:hasStageRequest", &request_id);
            pipeline.set_string("v-bpa:currentStage", "document_analysis", Lang::none());

            if let Err(e) = module.backend.mstorage_api.update_or_err(&module.ticket, event_id, "BPA", IndvOp::SetIn, &pipeline) {
                error!("Pipeline [{}]: failed to update status after recognition: {:?}", pipeline.get_id(), e);
                return Err(format!("Failed to update pipeline: {:?}", e).into());
            }
        },
        "document_analysis" => {
            let request_ids = pipeline.get_literals("v-bpa:hasStageRequest").unwrap_or_default();

            if request_ids.is_empty() {
                error!("Pipeline [{}]: no stage requests found for analysis stage", pipeline.get_id());
                return Err("Missing stage requests".into());
            }

            let request_id = &request_ids[0];
            let mut request = Individual::default();
            if module.backend.storage.get_individual(request_id, &mut request) != ResultCode::Ok {
                error!("Pipeline [{}]: failed to load analysis request [{}]", pipeline.get_id(), request_id);
                return Err(format!("Failed to load request [{}]", request_id).into());
            }

            let stage_progress = if request.any_exists("v-bpa:processingStatus", &["v-bpa:Completed"]) {
                1.0
            } else {
                // Check for failed status
                if request.any_exists("v-bpa:processingStatus", &["v-bpa:Failed"]) {
                    if let Some(error) = request.get_first_literal("v-bpa:lastError") {
                        error!("Pipeline [{}]: document analysis request [{}] failed with error: {}", pipeline.get_id(), request_id, error);
                        return Err(format!("Document analysis failed: {}", error).into());
                    } else {
                        error!("Pipeline [{}]: document analysis request [{}] failed without error details", pipeline.get_id(), request_id);
                        return Err("Document analysis failed without details".into());
                    }
                }
                0.0
            };

            update_pipeline_progress(event_id, module, &mut pipeline, "document_analysis", stage_progress, Some(request_ids.len()))?;

            if !request.any_exists("v-bpa:processingStatus", &["v-bpa:Completed"]) {
                info!("Pipeline [{}]: document analysis request [{}] in progress", pipeline.get_id(), request_id);
                return Ok(());
            }

            if let Some(result_id) = request.get_first_literal("v-bpa:hasResult") {
                info!("Pipeline [{}]: analysis completed, result document [{}]", pipeline.get_id(), result_id);

                let mut result_doc = Individual::default();
                if module.backend.storage.get_individual(&result_id, &mut result_doc) != ResultCode::Ok {
                    error!("Result doc [{}]: failed to load", result_id);
                    return Err(format!("Failed to load result doc [{}]", result_id).into());
                }
                result_doc.parse_all();
                let target_type = result_doc.get_first_literal("v-bpa:targetType").ok_or("fail read target type")?;

                let result_doc_id = format!("d:doc_{}", Uuid::new_v4());
                result_doc.set_id(&result_doc_id);
                result_doc.set_uri("rdf:type", &target_type);
                result_doc.set_uri("v-s:attachment", &pipeline.get_first_literal("v-s:attachment").ok_or("fail read attachment")?);
                result_doc.remove("v-bpa:targetType");
                if let Err(e) = module.backend.mstorage_api.update_or_err(&module.ticket, event_id, "BPA", IndvOp::Put, &result_doc) {
                    error!("Pipeline [{}]: failed to update result document: {:?}", result_doc_id, e);
                    return Err(format!("Failed to update result document: {:?}", e).into());
                }

                pipeline.set_uri("v-bpa:resultDocument", &result_doc_id);
                pipeline.set_uri("v-bpa:hasExecutionState", "v-bpa:ExecutionCompleted");
                pipeline.set_uri("v-bpa:processingStatus", "v-bpa:Completed");
                pipeline.set_datetime("v-bpa:endDate", Utc::now().timestamp());
                pipeline.remove("v-bpa:currentStage");
                pipeline.remove("v-bpa:hasStageRequest");
                pipeline.set_integer("v-bpa:percentComplete", 100);

                if let Err(e) = module.backend.mstorage_api.update_or_err(&module.ticket, event_id, "BPA", IndvOp::SetIn, &pipeline) {
                    error!("Pipeline [{}]: failed to update final status: {:?}", pipeline.get_id(), e);
                    return Err(format!("Failed to update pipeline: {:?}", e).into());
                }

                if let Err(e) = reset_document_status(module, &result_id, event_id) {
                    warn!("Pipeline [{}]: failed to reset status for document [{}]: {:?}", pipeline.get_id(), result_id, e);
                }

                info!("Pipeline [{}]: successfully completed", pipeline.get_id());
            }
        },
        _ => {
            error!("Pipeline [{}]: unknown stage [{}]", pipeline.get_id(), current_stage);
            return Err(format!("Invalid pipeline stage: {}", current_stage).into());
        },
    }

    Ok(())
}
