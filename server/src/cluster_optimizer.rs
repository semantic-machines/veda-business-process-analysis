use crate::common::extract_process_json;
use crate::prompt_manager::get_system_prompt;
use crate::queue_processor::BusinessProcessAnalysisModule;
use openai_dive::v1::resources::chat::{ChatCompletionParametersBuilder, ChatCompletionResponseFormat, ChatMessage, ChatMessageContent, JsonSchemaBuilder};
use serde::{Deserialize, Serialize};
use serde_json;
use tokio::runtime::Runtime;
use v_common::onto::datatype::Lang;
use v_common::onto::individual::Individual;
use v_common::v_api::api_client::IndvOp;
use v_common::v_api::obj::ResultCode;

/// Структура для оптимизированного процесса в соответствии с промптом
#[derive(Debug, Serialize, Deserialize)]
struct OptimizedProcess {
    name: String,
    optimization_proposal: String,
    recommended_frequency: i32,
    proposed_participants: Vec<String>,
    estimated_optimization_effect: i32,
    responsible_department: String,
    similarities: String,
    differences: String,
}

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
    let system_prompt = get_system_prompt(module, "v-bpa:OptimizeProcessesPrompt")?;

    // Создаем параметры запроса
    let parameters = prepare_optimization_parameters(module.model.clone(), system_prompt, analysis_data)?;

    // Отправляем запрос к AI
    info!("Sending optimization request to AI for cluster {}", cluster_id);
    let rt = Runtime::new()?;
    let optimization_result = rt.block_on(async { send_optimization_request(module, parameters).await })?;

    // Сохраняем результат оптимизации
    save_optimization_result(module, cluster_id, &optimization_result)?;

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

/// Подготавливает параметры запроса для оптимизации
fn prepare_optimization_parameters(
    model: String,
    system_prompt: String,
    analysis_data: serde_json::Value,
) -> Result<openai_dive::v1::resources::chat::ChatCompletionParameters, Box<dyn std::error::Error>> {
    let json_schema = serde_json::json!({
        "type": "object",
        "properties": {
            "optimized_process": {
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "Название оптимизированного процесса"
                    },
                    "optimization_proposal": {
                        "type": "string",
                        "description": "Краткое описание того, как этот оптимизированный процесс объединяет или упрощает исходные процессы"
                    },
                    "recommended_frequency": {
                        "type": "integer",
                        "description": "Предложенная частота выполнения процесса (в год)"
                    },
                    "proposed_participants": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Идеальные роли или лица, которые должны выполнять процесс"
                    },
                    "estimated_optimization_effect": {
                        "type": "integer",
                        "description": "Предполагаемые затраты времени на процесс (в часах)"
                    },
                    "responsible_department": {
                        "type": "string",
                        "description": "Отдел, который должен отвечать за выполнение процесса"
                    },
                    "similarities": {
                        "type": "string",
                        "description": "Сходства между исходными процессами"
                    },
                    "differences": {
                        "type": "string",
                        "description": "Различия между исходными процессами"
                    }
                },
                "required": [
                    "name",
                    "optimization_proposal",
                    "recommended_frequency",
                    "proposed_participants",
                    "estimated_optimization_effect",
                    "responsible_department",
                    "similarities",
                    "differences"
                ]
            }
        },
        "required": ["optimized_process"],
        "additionalProperties": false
    });

    let parameters = ChatCompletionParametersBuilder::default()
        .model(model)
        .messages(vec![
            ChatMessage::System {
                content: ChatMessageContent::Text(system_prompt),
                name: None,
            },
            ChatMessage::User {
                content: ChatMessageContent::Text(analysis_data.to_string()),
                name: None,
            },
        ])
        .response_format(ChatCompletionResponseFormat::JsonSchema(JsonSchemaBuilder::default().name("process_optimization").schema(json_schema).strict(true).build()?))
        .build()?;

    Ok(parameters)
}

/// Отправляет запрос на оптимизацию и получает результат
async fn send_optimization_request(
    module: &mut BusinessProcessAnalysisModule,
    parameters: openai_dive::v1::resources::chat::ChatCompletionParameters,
) -> Result<OptimizedProcess, Box<dyn std::error::Error>> {
    let result = module.client.chat().create(parameters).await?;

    if let Some(choice) = result.choices.first() {
        if let ChatMessage::Assistant {
            content: Some(ChatMessageContent::Text(text)),
            ..
        } = &choice.message
        {
            let response: serde_json::Value = serde_json::from_str(text)?;
            let optimized = serde_json::from_value(response["optimized_process"].clone())?;
            Ok(optimized)
        } else {
            error!("Unexpected message format in AI response");
            Err("Unexpected message format".into())
        }
    } else {
        error!("No response received from AI");
        Err("No response from AI".into())
    }
}

/// Сохраняет результат оптимизации
fn save_optimization_result(module: &mut BusinessProcessAnalysisModule, cluster_id: &str, optimization: &OptimizedProcess) -> Result<(), Box<dyn std::error::Error>> {
    let mut cluster = Individual::default();
    if module.backend.storage.get_individual(cluster_id, &mut cluster) != ResultCode::Ok {
        error!("Failed to load cluster {}", cluster_id);
        return Err(format!("Failed to load cluster {}", cluster_id).into());
    }
    cluster.parse_all();
    info!("Updating cluster {} with optimization results", cluster_id);

    // Обновляем данные кластера согласно онтологии и промпту
    cluster.set_string("v-bpa:proposedClusterName", &optimization.name, Lang::none());
    cluster.set_string("v-bpa:optimizationProposal", &optimization.optimization_proposal, Lang::none());
    cluster.set_string("v-bpa:clusterSimilarities", &optimization.similarities, Lang::none());
    cluster.set_string("v-bpa:clusterDifferences", &optimization.differences, Lang::none());
    cluster.set_string("v-bpa:proposedDepartment", &optimization.responsible_department, Lang::none());
    cluster.set_integer("v-bpa:proposedFrequency", optimization.recommended_frequency as i64);
    cluster.set_integer("v-bpa:estimatedLaborCost", optimization.estimated_optimization_effect as i64);

    // Очищаем и добавляем предлагаемых участников
    cluster.remove("v-bpa:proposedParticipants");
    for participant in &optimization.proposed_participants {
        cluster.add_string("v-bpa:proposedParticipants", participant, Lang::none());
    }

    // Сохраняем обновленный кластер
    if let Err(e) = module.backend.mstorage_api.update_or_err(&module.ticket, "", "BPA", IndvOp::Put, &mut cluster) {
        error!("Failed to update cluster {}: {:?}", cluster_id, e);
        return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("Failed to update cluster, err={:?}", e))));
    }

    info!("Successfully saved optimization results for cluster {}", cluster_id);
    Ok(())
}
