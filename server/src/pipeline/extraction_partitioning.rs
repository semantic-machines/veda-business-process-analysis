use crate::queue_processor::BusinessProcessAnalysisModule;
use v_common::onto::datatype::Lang;
use v_common::onto::individual::Individual;
use v_common::v_api::api_client::IndvOp;
use v_common::v_api::obj::ResultCode;

/// Pipeline that processes documents in 2 stages:
/// 1. Text extraction from source document (ImagesToTextPrompt)
/// 2. Document analysis and summarization (DocumentAnalysisPrompt)
pub fn process_extraction_summarization(module: &mut BusinessProcessAnalysisModule, request: &mut Individual) -> Result<(), Box<dyn std::error::Error>> {
    // Check if this is text extraction request
    if !request.any_exists("v-bpa:prompt", &["v-bpa:ImagesToTextPrompt"]) {
        return Ok(());
    }

    // Check processing status
    if !request.any_exists("v-bpa:processingStatus", &["v-bpa:Completed"]) {
        return Ok(());
    }

    // Get result individual containing extracted text
    let result_id = request.get_first_literal("v-bpa:hasResult").ok_or("No result found")?;
    let mut result_individual = Individual::default();

    if module.backend.storage.get_individual(&result_id, &mut result_individual) != ResultCode::Ok {
        error!("Failed to load result individual {}", result_id);
        return Err("Failed to load result individual".into());
    }

    // Get extracted text and target type
    let extracted_text = result_individual.get_literals("v-bpa:extractedText").ok_or("No extracted text found")?.join("");
    let target_type = result_individual.get_first_literal("v-bpa:targetType").ok_or("No target type found")?;

    // Create new document analysis request
    let mut doc_request = Individual::default();
    doc_request.set_id(&format!("d:request_{}", uuid::Uuid::new_v4()));
    doc_request.set_uri("rdf:type", "v-bpa:GenericProcessingRequest");
    doc_request.set_uri("v-bpa:prompt", "v-bpa:DocumentAnalysisPrompt");
    doc_request.set_string("v-bpa:rawInput", &extracted_text, Lang::none());
    doc_request.set_string("v-bpa:targetType", &target_type, Lang::none());

    // Save new request
    if let Err(e) = module.backend.mstorage_api.update_or_err(&module.ticket, "", "", IndvOp::Put, &mut doc_request) {
        error!("Failed to create document analysis request: {:?}", e);
        return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("Failed to create request, err={:?}", e))));
    }

    info!("Created document analysis request {} from text extraction result", doc_request.get_id());
    Ok(())
}
