/// Обработчик для выполнения произвольных операций с индивидами на основе пользовательского ввода
/// и заданного типа целевого индивида.
use crate::common::{
    convert_full_to_short_predicates, convert_short_to_full_predicates, load_schema, prepare_request_ai_parameters, send_request_to_ai,
    set_to_individual_from_ai_response,
};
use crate::queue_processor::BusinessProcessAnalysisModule;
use crate::types::PropertyMapping;
use serde_json::Value;
use tokio::runtime::Runtime;
use v_common::onto::datatype::Lang;
use v_common::onto::individual::Individual;
use v_common::v_api::api_client::IndvOp;
use v_common::v_api::obj::ResultCode;
//use crate::types::SYSTEM_PREDICATE;

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

    let mut property_mapping = PropertyMapping::new();
    let property_schema = load_schema(module, &prompt_id, None, &mut property_mapping)?;

    // Обрабатываем входные данные, если они есть
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
    // Подготавливаем данные для анализа
    let analysis_data = prepare_analysis_data(&raw_input, &mut target_type_def, structured_input)?;

    // Создаем параметры запроса и получаем маппинг свойств
    let req_to_ai = prepare_request_ai_parameters(module, &prompt_id, analysis_data, property_schema, &mut property_mapping)?;

    info!("@E = req_to_ai={:?}", req_to_ai);

    // Отправляем запрос к AI
    info!("Sending request to AI for processing input: {}", raw_input);
    let rt = Runtime::new()?;
    let ai_response = rt.block_on(async { send_request_to_ai(module, req_to_ai).await })?;

    info!("@G ai_response={:?}", ai_response);

    if is_structured_input {
        if is_structured_input {
            if let Some(result) = ai_response.get("result") {
                // Преобразуем короткие имена и человекочитаемые значения обратно в URI
                let mapped_result = convert_short_to_full_predicates(result, &property_mapping)?;
                request.set_string("v-bpa:structuredOutput", &mapped_result.to_string(), Lang::none());
            }
        }
    } else {
        // Создаем новый индивид целевого типа для сохранения результата
        let result_id = format!("d:generic_result_{}", uuid::Uuid::new_v4());
        let mut result_individual = Individual::default();
        result_individual.set_id(&result_id);
        result_individual.set_uri("rdf:type", "v-bpa:GenericProcessingResult");
        result_individual.set_uri("v-bpa:targetType", &target_type);

        // Сохраняем результат анализа AI, включая очищенный текст
        set_to_individual_from_ai_response(module, &mut result_individual, &ai_response, &property_mapping)?;

        // Сохраняем обновленный индивид
        if let Err(e) = module.backend.mstorage_api.update_or_err(&module.ticket, "", "BPA", IndvOp::Put, &mut result_individual) {
            error!("Failed to update individual {}: {:?}", result_individual.get_id(), e);
            return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("Failed to update individual, err={:?}", e))));
        }

        // Обновляем исходный запрос, добавляя ссылку на созданный результат
        request.set_uri("v-bpa:hasResult", &result_id);
    }

    request.set_uri("v-bpa:processingStatus", "v-bpa:Completed");

    if let Err(e) = module.backend.mstorage_api.update_or_err(&module.ticket, "", "BPA", IndvOp::Put, request) {
        error!("Failed to update request {}: {:?}", request.get_id(), e);
        return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("Failed to update request, err={:?}", e))));
    }

    info!("Successfully processed generic request {} ", request.get_id());
    Ok(())
}
