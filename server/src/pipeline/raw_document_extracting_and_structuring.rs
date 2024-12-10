use crate::document_status_handler::reset_document_status;
use crate::queue_processor::BusinessProcessAnalysisModule;
use chrono::Utc;
use uuid::Uuid;
use v_common::onto::datatype::Lang;
use v_common::onto::individual::Individual;
use v_common::v_api::api_client::IndvOp;
use v_common::v_api::obj::ResultCode;

pub fn raw_document_extracting_and_structuring(module: &mut BusinessProcessAnalysisModule, pipeline: &mut Individual) -> Result<(), Box<dyn std::error::Error>> {
    info!("=== Starting Document Processing Pipeline ===");
    info!("Pipeline request ID: {}", pipeline.get_id());

    // Get current stage
    let current_stage = pipeline.get_first_literal("v-bpa:currentStage").unwrap_or_default();

    match current_stage.as_str() {
        "" => {
            // Initial stage - create text extraction request
            info!("Initializing document processing pipeline...");

            // Get attachment
            let attachment_id = pipeline.get_first_literal("v-s:attachment").ok_or("No attachment found in pipeline")?;

            // Create text extraction request
            let mut request = Individual::default();
            let request_id = format!("d:request_{}", Uuid::new_v4());
            request.set_id(&request_id);
            request.set_uri("rdf:type", "v-bpa:GenericProcessingRequest");
            request.set_uri("v-bpa:prompt", "v-bpa:ImagesToTextPrompt");
            request.set_uri("v-s:attachment", &attachment_id);
            request.set_uri("v-s:hasParentLink", pipeline.get_id());

            // Save request
            if let Err(e) = module.backend.mstorage_api.update_or_err(&module.ticket, pipeline.get_id(), "PIPELINE", IndvOp::Put, &mut request) {
                error!("Failed to create text extraction request: {:?}", e);
                return Err(format!("Failed to create request: {:?}", e).into());
            }

            // Update pipeline status
            pipeline.set_string("v-bpa:currentStage", "text_extraction", Lang::none());
            pipeline.set_uri("v-bpa:hasExecutionState", "v-bpa:ExecutionInProgress");
            pipeline.set_uri("v-bpa:hasNextStage", &request_id);
            pipeline.set_datetime("v-bpa:startDate", Utc::now().timestamp());

            if let Err(e) = module.backend.mstorage_api.update_or_err(&module.ticket, pipeline.get_id(), "", IndvOp::SetIn, pipeline) {
                error!("Failed to update pipeline status: {:?}", e);
                return Err(format!("Failed to update pipeline: {:?}", e).into());
            }
        },
        "text_extraction" => {
            // Check if text extraction completed
            if let Some(next_stage_id) = pipeline.get_first_literal("v-bpa:hasNextStage") {
                let mut next_stage = Individual::default();
                if module.backend.storage.get_individual(&next_stage_id, &mut next_stage) != ResultCode::Ok {
                    return Err(format!("Failed to load next stage: {}", next_stage_id).into());
                }

                if next_stage.any_exists("v-bpa:processingStatus", &["v-bpa:Completed"]) {
                    // Create document analysis request
                    info!("Text extraction completed, creating document analysis request...");
                    let result_id = next_stage.get_first_literal("v-bpa:hasResult").ok_or("No result found in text extraction request")?;

                    let mut res_indv = module.backend.get_individual_s(&result_id).ok_or("fail load v-bpa:hasResult form result")?;

                    let mut doc_request = Individual::default();
                    let doc_request_id = format!("d:request_{}", Uuid::new_v4());
                    doc_request.set_id(&doc_request_id);
                    doc_request.set_uri("rdf:type", "v-bpa:GenericProcessingRequest");
                    doc_request.set_uri("v-bpa:prompt", "v-bpa:DocumentAnalysisPrompt");
                    doc_request.set_uri("v-s:hasParentLink", pipeline.get_id());
                    doc_request.set_string(
                        "v-bpa:rawInput",
                        &res_indv.get_first_literal("v-bpa:extractedText").ok_or("fail load v-bpa:hasResult->v-bpa:extractedText")?,
                        Lang::none(),
                    );

                    if let Err(e) = module.backend.mstorage_api.update_or_err(&module.ticket, pipeline.get_id(), "PIPELINE", IndvOp::Put, &mut doc_request) {
                        error!("Failed to create document analysis request: {:?}", e);
                        return Err(format!("Failed to create request: {:?}", e).into());
                    }

                    // Update pipeline status
                    pipeline.set_string("v-bpa:currentStage", "document_analysis", Lang::none());
                    pipeline.set_uri("v-bpa:hasNextStage", &doc_request_id);

                    if let Err(e) = module.backend.mstorage_api.update_or_err(&module.ticket, "", "BPA", IndvOp::SetIn, pipeline) {
                        error!("Failed to update pipeline status: {:?}", e);
                        return Err(format!("Failed to update pipeline: {:?}", e).into());
                    }
                }
            }
        },
        "document_analysis" => {
            // Check if document analysis completed
            if let Some(next_stage_id) = pipeline.get_first_literal("v-bpa:hasNextStage") {
                let mut next_stage = Individual::default();
                if module.backend.storage.get_individual(&next_stage_id, &mut next_stage) != ResultCode::Ok {
                    return Err(format!("Failed to load next stage: {}", next_stage_id).into());
                }

                if next_stage.any_exists("v-bpa:processingStatus", &["v-bpa:Completed"]) {
                    info!("Document analysis completed, finalizing pipeline...");

                    // Get result document
                    let result_id = next_stage.get_first_literal("v-bpa:hasResult").ok_or("No result found in document analysis request")?;

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
