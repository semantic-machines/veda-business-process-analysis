use crate::queue_processor::BusinessProcessAnalysisModule;
use v_common::onto::datatype::Lang;
use v_common::onto::individual::Individual;
use v_common::v_api::api_client::IndvOp;
use v_common::v_api::obj::ResultCode;

/// Pipeline that processes documents in 2 stages:
/// 1. Text extraction from source document (ImagesToTextPrompt)
/// 2. Document analysis and partitioning (DocumentAnalysisPrompt)
pub fn process_extraction_and_partitioning(module: &mut BusinessProcessAnalysisModule, request: &mut Individual) -> Result<(), Box<dyn std::error::Error>> {
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

/// Creates an individual of target type from document analysis results
pub fn create_target_individual(module: &mut BusinessProcessAnalysisModule, request: &mut Individual) -> Result<(), Box<dyn std::error::Error>> {
    // Check if this is document analysis request
    if !request.any_exists("v-bpa:prompt", &["v-bpa:DocumentAnalysisPrompt"]) {
        return Ok(());
    }

    // Check processing status
    if !request.any_exists("v-bpa:processingStatus", &["v-bpa:Completed"]) {
        return Ok(());
    }

    // Get result of document analysis
    let result_id = request.get_first_literal("v-bpa:hasResult").ok_or("No analysis result found")?;
    let mut result_individual = Individual::default();

    if module.backend.storage.get_individual(&result_id, &mut result_individual) != ResultCode::Ok {
        error!("Failed to load analysis result {}", result_id);
        return Err("Failed to load analysis result".into());
    }
    result_individual.parse_all();

    // Get target type
    let target_type = result_individual.get_first_literal("v-bpa:targetType").ok_or("No target type specified")?;

    // Create new individual of target type
    let target_id = format!("d:{}_{}", target_type.split(':').last().unwrap_or("item"), uuid::Uuid::new_v4());
    let mut target_individual = Individual::new_from_obj(result_individual.get_obj());
    target_individual.set_id(&target_id);
    target_individual.set_uri("rdf:type", &target_type);
    target_individual.remove("v-bpa:targetType");

    // Save target individual
    if let Err(e) = module.backend.mstorage_api.update_or_err(&module.ticket, "", "BPA", IndvOp::Put, &mut target_individual) {
        error!("Failed to save target individual: {:?}", e);
        return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("Failed to save target individual, err={:?}", e))));
    }

    info!("Created target individual {} of type {}", target_id, target_type);

    // Add reference to created individual in original request
    request.add_uri("v-bpa:hasTargetIndividual", &target_id);
    if let Err(e) = module.backend.mstorage_api.update_or_err(&module.ticket, "", "BPA", IndvOp::SetIn, request) {
        error!("Failed to update request with target individual reference: {:?}", e);
        return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("Failed to update request, err={:?}", e))));
    }

    Ok(())
}
