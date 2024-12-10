use crate::ai_client::{save_to_interaction_file, send_structured_request_to_ai};
/// Обработчик для выполнения произвольных операций с индивидами на основе пользовательского ввода
/// и заданного типа целевого индивида.
use crate::common::{
    convert_full_to_short_predicates, convert_short_to_full_predicates, load_schema, prepare_request_ai_parameters, set_to_individual_from_ai_response, ClientType,
};
use crate::process_structured_schema;
use crate::queue_processor::BusinessProcessAnalysisModule;
use crate::types::PropertyMapping;
use serde_json::Value;
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
        process_structured_schema::process_structured_schema(module, request, &mut prompt_individual)?;
    } else {
        process_ontology_input(module, request, &mut prompt_individual)?;
    }

    info!("Successfully processed generic request {} ", request.get_id());
    Ok(())
}
