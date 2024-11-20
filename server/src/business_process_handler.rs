// business_process_handler.rs

use crate::common::{extract_process_json, prepare_request_ai_parameters, send_request_to_ai, set_to_individual_from_ai_response};
use crate::queue_processor::BusinessProcessAnalysisModule;
use std::collections::HashSet;
use std::io;
use tokio::runtime::Runtime;
use v_common::onto::individual::Individual;
use v_common::v_api::api_client::IndvOp;

/// Анализирует обоснованность бизнес-процесса на основе связанных документов
/// используя AI для оценки уровня обоснованности.
///
/// # Arguments
/// * `module` - Модуль анализа бизнес-процессов с настройками и клиентом AI
/// * `bp_obj` - Индивид бизнес-процесса для анализа
///
/// # Returns
/// * `Result<(), Box<dyn std::error::Error>>` - Результат анализа и сохранения оценки
pub fn analyze_process_justification(module: &mut BusinessProcessAnalysisModule, bp_obj: &mut Individual) -> Result<(), Box<dyn std::error::Error>> {
    bp_obj.parse_all();

    // Check if process documents exist
    let has_documents = bp_obj.get_literals("v-bpa:hasProcessDocument").map_or(false, |j| !j.is_empty());

    if !has_documents {
        info!("Process {} has no justification documents. Setting status to NoDocumentForJustification", bp_obj.get_id());

        // Set the process justification status
        bp_obj.set_uri("v-bpa:hasProcessJustification", "v-bpa:NoDocumentForJustification");

        // Save the updated individual to storage
        if let Err(e) = module.backend.mstorage_api.update_or_err(&module.ticket, "", "BPA", IndvOp::Put, bp_obj) {
            error!("Failed to update individual {}: {:?}", bp_obj.get_id(), e);
            return Err(Box::new(std::io::Error::new(io::ErrorKind::Other, format!("Failed to update individual, err={:?}", e))));
        }

        return Ok(());
    }

    // Continue with existing analysis if justification documents are present
    let process_json = extract_process_json(bp_obj, module)?;

    info!("Process Name: {}", process_json["processName"]);

    // Подготавливаем параметры запроса и получаем маппинг свойств
    let (parameters, property_mapping) =
        prepare_request_ai_parameters(module, "v-bpa:AnalyzeBusinessPrompt", process_json, Some(HashSet::from(["v-bpa:NoDocumentForJustification"])))?;
    debug!("Parameters prepared for OpenAI: {:?}", parameters);

    // Создаем новый рантайм для асинхронного выполнения
    let rt = Runtime::new()?;

    // Отправляем запрос к AI
    let ai_response = rt.block_on(async { send_request_to_ai(module, parameters).await })?;

    // Сохраняем результат в индивиде с учетом маппинга свойств
    set_to_individual_from_ai_response(module, bp_obj, &ai_response, &property_mapping)?;

    // Сохраняем обновленный индивид в хранилище
    if let Err(e) = module.backend.mstorage_api.update_or_err(&module.ticket, "", "BPA", IndvOp::Put, bp_obj) {
        error!("Failed to update individual {}: {:?}", bp_obj.get_id(), e);
        return Err(Box::new(std::io::Error::new(io::ErrorKind::Other, format!("Failed to update individual, err={:?}", e))));
    }

    Ok(())
}
