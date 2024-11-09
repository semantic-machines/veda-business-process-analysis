use crate::common::{extract_process_json, prepare_request_ai_parameters, save_individual_from_ai_response, send_request_to_ai};
use crate::queue_processor::BusinessProcessAnalysisModule;
use serde_json;
use tokio::runtime::Runtime;
use v_common::onto::individual::Individual;
use v_common::v_api::obj::ResultCode;

/// Анализирует кластер процессов и предлагает оптимизацию
pub fn analyze_and_optimize_cluster(module: &mut BusinessProcessAnalysisModule, cluster_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting cluster optimization analysis for cluster: {}", cluster_id);

    // Загружаем кластер
    let mut cluster = Individual::default();
    if module.backend.storage.get_individual(cluster_id, &mut cluster) != ResultCode::Ok {
        error!("Failed to load cluster {}", cluster_id);
        return Err(format!("Failed to load cluster {}", cluster_id).into());
    }

    // Получаем список процессов в кластере
    let process_ids = cluster.get_literals("v-bpa:hasProcess").unwrap_or_default();
    if process_ids.is_empty() {
        info!("No processes found in cluster {}", cluster_id);
        return Ok(());
    }

    // Загружаем данные всех процессов
    let mut processes_data = Vec::new();
    for process_id in &process_ids {
        let mut process = Individual::default();
        if module.backend.storage.get_individual(process_id, &mut process) != ResultCode::Ok {
            error!("Failed to load process {}", process_id);
            continue;
        }
        process.parse_all();
        if let Ok(data) = extract_process_json(&mut process, module) {
            processes_data.push(data);
        }
    }

    if processes_data.is_empty() {
        error!("No valid process data found in cluster {}", cluster_id);
        return Ok(());
    }

    // Подготавливаем данные для анализа
    info!("Preparing optimization data for {} processes", processes_data.len());
    let analysis_data = prepare_optimization_data(&processes_data)?;

    // Создаем параметры запроса и получаем маппинг свойств
    let (parameters, property_mapping) = prepare_request_ai_parameters(module, "v-bpa:OptimizeProcessesPrompt", analysis_data)?;

    // Отправляем запрос к AI
    info!("Sending optimization request to AI for cluster {}", cluster_id);
    let rt = Runtime::new()?;
    let optimization_result = rt.block_on(async { send_request_to_ai(module, parameters).await })?;

    info!("@ optimization_result={:?}", optimization_result);
    // Сохраняем результат оптимизации с учетом маппинга
    save_individual_from_ai_response(module, Some(cluster_id), None, &optimization_result, &property_mapping)?;

    info!("Successfully completed optimization analysis for cluster {}", cluster_id);
    Ok(())
}

/// Подготавливает данные процессов для анализа оптимизации
fn prepare_optimization_data(processes: &[serde_json::Value]) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    Ok(serde_json::json!({
        "processes": processes,
        "count": processes.len()
    }))
}
