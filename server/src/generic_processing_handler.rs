use crate::common::{prepare_request_ai_parameters, send_request_to_ai, set_to_individual_from_ai_response};
use crate::queue_processor::BusinessProcessAnalysisModule;
use serde_json::Value;
use tokio::runtime::Runtime;
use v_common::onto::individual::Individual;
use v_common::v_api::api_client::IndvOp;
use v_common::v_api::obj::ResultCode;

/// Обработчик для выполнения произвольных операций с индивидами на основе пользовательского ввода
/// и заданного типа целевого индивида.
pub fn process_generic_request(module: &mut BusinessProcessAnalysisModule, request: &mut Individual) -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting generic request processing for request: {}", request.get_id());

    // Получаем пользовательский ввод
    let raw_input = request.get_first_literal("v-bpa:rawInput").ok_or("No raw input provided")?;

    // Получаем ссылку на промпт и загружаем его
    let prompt_id = request.get_first_literal("v-bpa:prompt").ok_or("No prompt specified")?;

    let mut prompt_individual = Individual::default();
    if module.backend.storage.get_individual(&prompt_id, &mut prompt_individual) != ResultCode::Ok {
        return Err(format!("Failed to load prompt: {}", prompt_id).into());
    }
    // Получаем тип целевого индивида
    let target_type = prompt_individual.get_first_literal("v-bpa:targetType").ok_or("No target type specified")?;

    // Загружаем определение целевого типа из онтологии
    let mut target_type_def = Individual::default();
    if module.backend.storage.get_individual(&target_type, &mut target_type_def) != ResultCode::Ok {
        return Err(format!("Failed to load target type definition: {}", target_type).into());
    }
    target_type_def.parse_all();

    // Подготавливаем данные для анализа
    let analysis_data = prepare_analysis_data(&raw_input, &mut target_type_def)?;

    // Создаем параметры запроса и получаем маппинг свойств
    let (parameters, property_mapping) = prepare_request_ai_parameters(module, &prompt_id, analysis_data)?;

    // Отправляем запрос к AI
    info!("Sending request to AI for processing input: {}", raw_input);
    let rt = Runtime::new()?;
    let ai_response = rt.block_on(async { send_request_to_ai(module, parameters).await })?;

    // Создаем новый индивид целевого типа для сохранения результата
    let result_id = format!("d:generic_result_{}", uuid::Uuid::new_v4());
    let mut result_individual = Individual::default();
    result_individual.set_id(&result_id);
    result_individual.set_uri("rdf:type", "v-bpa:GenericProcessingResult");
    result_individual.set_uri("v-bpa:targetType", &target_type);

    // Сохраняем оригинальный текст
    //result_individual.set_string("v-bpa:originalInput", &raw_input, Lang::none());

    // Сохраняем результат анализа AI, включая очищенный текст
    set_to_individual_from_ai_response(module, &mut result_individual, &ai_response, &property_mapping)?;

    // Сохраняем обновленный индивид
    if let Err(e) = module.backend.mstorage_api.update_or_err(&module.ticket, "", "BPA", IndvOp::Put, &mut result_individual) {
        error!("Failed to update individual {}: {:?}", result_individual.get_id(), e);
        return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("Failed to update individual, err={:?}", e))));
    }

    // Обновляем исходный запрос, добавляя ссылку на созданный результат
    request.set_uri("v-bpa:hasResult", &result_id);
    request.set_uri("v-bpa:processingStatus", "v-bpa:Completed");

    if let Err(e) = module.backend.mstorage_api.update_or_err(&module.ticket, "", "BPA", IndvOp::Put, request) {
        error!("Failed to update request {}: {:?}", request.get_id(), e);
        return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("Failed to update request, err={:?}", e))));
    }

    info!("Successfully processed generic request {} and created result {}", request.get_id(), result_id);
    Ok(())
}

/// Подготавливает данные для анализа на основе пользовательского ввода,
/// определения целевого типа и промпта
fn prepare_analysis_data(raw_input: &str, target_type_def: &mut Individual) -> Result<Value, Box<dyn std::error::Error>> {
    Ok(serde_json::json!({
        "input": raw_input,
        "targetType": {
            "id": target_type_def.get_id(),
            "label": target_type_def.get_first_literal("rdfs:label")
        }
    }))
}
