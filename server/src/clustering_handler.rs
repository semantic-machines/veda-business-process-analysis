// clustering_handler.rs

use openai_dive::v1::resources::chat::{ChatCompletionParametersBuilder, ChatCompletionResponseFormat, ChatMessage, ChatMessageContent, JsonSchemaBuilder};
use serde_json;
use tokio::runtime::Runtime;
use v_common::onto::individual::Individual;
use v_common::v_api::api_client::IndvOp;
use v_common::v_api::obj::ResultCode;

use crate::queue_processor::BusinessProcessAnalysisModule;
use crate::prompt_manager::get_system_prompt;

/// Анализирует группу бизнес-процессов и определяет кластеры схожих процессов
///
/// # Arguments
/// * `module` - Модуль анализа с настройками и клиентом AI
/// * `clustering_attempt` - Индивид, описывающий попытку кластеризации
///
/// # Returns
/// * `Result<(), Box<dyn std::error::Error>>` - Результат кластеризации
pub fn analyze_process_clusters(
    module: &mut BusinessProcessAnalysisModule,
    clustering_attempt: &mut Individual,
) -> Result<(), Box<dyn std::error::Error>> {
    // Получаем список всех бизнес-процессов для анализа
    let mut processes = collect_business_processes(module, clustering_attempt)?;
    
    // Получаем системный промпт для кластеризации
    let mut system_prompt = get_system_prompt(module, "v-bpa:ClusterizeProcessesPrompt")?;
    system_prompt.push_str("\nПожалуйста, верни ответ в формате JSON, соответствующий указанной схеме.");

    let processes_data = prepare_processes_data(&mut processes)?;
    
    let chat_parameters = prepare_clustering_parameters(
        module.model.clone(),
        system_prompt,
        processes_data,
    )?;

    let rt = Runtime::new()?;
    rt.block_on(async {
        let clusters = send_clustering_request(module, chat_parameters).await?;
        save_clustering_results(module, clustering_attempt, &clusters)?;
        Ok(())
    })
}

/// Собирает все бизнес-процессы, подлежащие кластеризации
fn collect_business_processes(
    module: &mut BusinessProcessAnalysisModule,
    clustering_attempt: &mut Individual,
) -> Result<Vec<Individual>, Box<dyn std::error::Error>> {
    let mut processes = Vec::new();
    
    // Получаем список ID процессов для кластеризации
    let process_refs = clustering_attempt.get_literals("v-bpa:processToAnalyze").unwrap_or_default();
    
    for process_id in process_refs {
        let mut process = Individual::default();
        if module.backend.storage.get_individual(&process_id, &mut process) == ResultCode::Ok {
            process.parse_all();
            processes.push(process);
        } else {
            error!("Не удалось загрузить бизнес-процесс с ID: {}", process_id);
        }
    }

    Ok(processes)
}

/// Подготавливает данные о процессах для анализа AI
fn prepare_processes_data(processes: &mut [Individual]) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let processes_data: Vec<serde_json::Value> = processes
        .iter_mut()
        .map(|mut process| {
            serde_json::json!({
                "id": process.get_id(),
                "name": process.get_first_literal("v-bpa:processName").unwrap_or_default(),
                "description": process.get_first_literal("v-bpa:processDescription").unwrap_or_default(),
                "participants": process.get_first_literal("v-bpa:processParticipant").unwrap_or_default(),
                "department": process.get_first_literal("v-bpa:responsibleDepartment").unwrap_or_default(),
                "frequency": process.get_first_literal("v-bpa:processFrequency").unwrap_or_default(),
            })
        })
        .collect();

    Ok(serde_json::json!({
        "processes": processes_data
    }))
}

/// Подготавливает параметры запроса для кластеризации
fn prepare_clustering_parameters(
    model: String,
    system_prompt: String,
    processes_data: serde_json::Value,
) -> Result<openai_dive::v1::resources::chat::ChatCompletionParameters, Box<dyn std::error::Error>> {
    let json_schema = serde_json::json!({
        "type": "object",
        "properties": {
            "clusters": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "cluster_name": { "type": "string" },
                        "cluster_description": { "type": "string" },
                        "process_ids": {
                            "type": "array",
                            "items": { "type": "string" }
                        },
                        "similarity_factors": {
                            "type": "array",
                            "items": { "type": "string" }
                        }
                    },
                    "required": ["cluster_name", "cluster_description", "process_ids", "similarity_factors"]
                }
            }
        },
        "required": ["clusters"]
    });

    let parameters = ChatCompletionParametersBuilder::default()
        .model(model)
        .messages(vec![
            ChatMessage::System {
                content: ChatMessageContent::Text(system_prompt),
                name: None,
            },
            ChatMessage::User {
                content: ChatMessageContent::Text(processes_data.to_string()),
                name: None,
            },
        ])
        .response_format(ChatCompletionResponseFormat::JsonSchema(
            JsonSchemaBuilder::default()
                .name("process_clusters")
                .schema(json_schema)
                .strict(true)
                .build()?,
        ))
        .build()?;

    Ok(parameters)
}

/// Отправляет запрос на кластеризацию к AI
async fn send_clustering_request(
    module: &mut BusinessProcessAnalysisModule,
    parameters: openai_dive::v1::resources::chat::ChatCompletionParameters,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    info!("Sending clustering request to OpenAI API");

    let result = module.client.chat().create(parameters).await?;
    debug!("Received clustering response from OpenAI: {:?}", result);

    if let Some(choice) = result.choices.first() {
        if let ChatMessage::Assistant {
            content: Some(ChatMessageContent::Text(text)),
            ..
        } = &choice.message
        {
            info!("Received clustering text response from OpenAI: {}", text);
            let clusters: serde_json::Value = serde_json::from_str(text)?;
            Ok(clusters)
        } else {
            Err("Unexpected message format in response".into())
        }
    } else {
        Err("No choices in the response".into())
    }
}

/// Сохраняет результаты кластеризации
fn save_clustering_results(
    module: &mut BusinessProcessAnalysisModule,
    clustering_attempt: &mut Individual,
    clusters: &serde_json::Value,
) -> Result<(), Box<dyn std::error::Error>> {
    // Сохраняем результаты кластеризации как JSON

    Ok(())
}
