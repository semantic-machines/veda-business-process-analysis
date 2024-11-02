use openai_dive::v1::resources::chat::{ChatCompletionParametersBuilder, ChatCompletionResponseFormat, ChatMessage, ChatMessageContent, JsonSchemaBuilder};
use serde_json;
use std::collections::{HashMap, HashSet};
use std::{io, thread, time};
use tokio::runtime::Runtime;
use v_common::onto::individual::Individual;
use v_common::v_api::api_client::IndvOp;
use v_common::v_api::obj::ResultCode;
use crate::common::extract_process_json;
use crate::prompt_manager::get_system_prompt;
use crate::queue_processor::BusinessProcessAnalysisModule;
use v_common::onto::datatype::Lang;
use v_common::search::common::{FTQuery, QueryResult};

/// Анализирует бизнес-процессы и определяет кластеры схожих процессов
pub fn analyze_process_clusters(module: &mut BusinessProcessAnalysisModule, clustering_attempt: &mut Individual) -> Result<(), Box<dyn std::error::Error>> {
    // Проверяем текущий статус
    let status = if let Some(statuses) = clustering_attempt.get_literals("v-bpa:clusterizationStatus") {
        statuses.first().cloned().unwrap_or_default()
    } else {
        String::new()
    };

    match status.as_str() {
        "" => {
            info!("Starting new clustering attempt: {}", clustering_attempt.get_id());
            initialize_clustering(module, clustering_attempt)?;
        },
        "v-bpa:ComparingPairs" => {
            info!("Continuing pair comparison for attempt: {}", clustering_attempt.get_id());
            compare_next_pair(module, clustering_attempt)?;
        },
        "v-bpa:PairsCompared" => {
            info!("Building clusters for attempt: {}", clustering_attempt.get_id());
            build_clusters(module, clustering_attempt)?;
        },
        "v-bpa:Completed" => {
            info!("Clustering already completed for attempt: {}", clustering_attempt.get_id());
            return Ok(());
        },
        _ => {
            error!("Unknown clustering status: {}", status);
            return Err("Invalid clustering status".into());
        },
    }

    Ok(())
}

/// Находит все бизнес-процессы в системе
fn find_all_business_processes(module: &mut BusinessProcessAnalysisModule) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut res = QueryResult::default();
    res.result_code = ResultCode::NotReady;

    // Пытаемся получить результаты, повторяем при ошибках синхронизации
    while res.result_code == ResultCode::NotReady || res.result_code == ResultCode::DatabaseModifiedError {
        res = module.xr.query(FTQuery::new_with_user("cfg:VedaSystem", "'rdf:type' === 'v-bpa:BusinessProcess'"), &mut module.backend.storage);

        if res.result_code == ResultCode::InternalServerError {
            return Err(io::Error::new(io::ErrorKind::Other, format!("Search failed with error: {:?}", res.result_code)).into());
        }

        if res.result_code != ResultCode::Ok {
            warn!("Failed to search business processes, retry in 3 seconds...");
            thread::sleep(time::Duration::from_secs(3));
        }
    }

    let mut process_ids = Vec::new();
    if res.result_code == ResultCode::Ok && res.count > 0 {
        process_ids.extend(res.result);
        info!("Found {} business processes", process_ids.len());
    } else {
        info!("No business processes found");
    }

    Ok(process_ids)
}

/// Инициализирует процесс кластеризации
fn initialize_clustering(module: &mut BusinessProcessAnalysisModule, clustering_attempt: &mut Individual) -> Result<(), Box<dyn std::error::Error>> {
    // Получаем список всех бизнес-процессов
    let process_ids = find_all_business_processes(module)?;

    if process_ids.is_empty() {
        return Err("No business processes found for clustering".into());
    }

    // Сохраняем список процессов
    let process_len = process_ids.len();
    clustering_attempt.set_uris("v-bpa:processesToAnalyze", process_ids);

    // Инициализируем прогресс сравнения
    clustering_attempt.set_string(
        "v-bpa:currentPairIndex",
        "0,1", // Начинаем с первой пары
        Lang::none(),
    );

    // Устанавливаем статус
    clustering_attempt.set_uri("v-bpa:clusterizationStatus", "v-bpa:ComparingPairs");

    update_individual(module, clustering_attempt)?;
    info!("Initialized clustering attempt {} with {} processes", clustering_attempt.get_id(), process_len);

    Ok(())
}

/// Сравнивает следующую пару процессов
fn compare_next_pair(module: &mut BusinessProcessAnalysisModule, clustering_attempt: &mut Individual) -> Result<(), Box<dyn std::error::Error>> {
    // Получаем текущие индексы
    let current_pair = if let Some(pairs) = clustering_attempt.get_literals("v-bpa:currentPairIndex") {
        pairs.first().cloned().ok_or("No current pair index found")?
    } else {
        return Err("No current pair index found".into());
    };

    let indices: Vec<usize> = current_pair.split(',').map(|s| s.parse::<usize>()).collect::<Result<Vec<_>, _>>()?;

    let processes = clustering_attempt.get_literals("v-bpa:processesToAnalyze").ok_or("No processes to analyze found")?;

    if indices[0] >= processes.len() || indices[1] >= processes.len() {
        // Все пары сравнены
        clustering_attempt.set_uri("v-bpa:clusterizationStatus", "v-bpa:PairsCompared");
        update_individual(module, clustering_attempt)?;
        info!("Completed comparing all pairs for attempt: {}", clustering_attempt.get_id());
        return Ok(());
    }

    info!("Comparing processes {} and {}", processes[indices[0]], processes[indices[1]]);

    // Сравниваем текущую пару
    let is_similar = compare_processes(module, &processes[indices[0]], &processes[indices[1]])?;

    if is_similar {
        // Сохраняем похожую пару
        let pair = format!("{},{}", processes[indices[0]], processes[indices[1]]);
        clustering_attempt.set_string("v-bpa:similarPairs", &pair, Lang::none());
        info!("Found similar processes: {}", pair);
    }

    // Вычисляем следующую пару
    let (next_i, next_j) = if indices[1] + 1 < processes.len() {
        (indices[0], indices[1] + 1)
    } else {
        (indices[0] + 1, indices[0] + 2)
    };

    // Обновляем индексы
    clustering_attempt.set_string("v-bpa:currentPairIndex", &format!("{},{}", next_i, next_j), Lang::none());

    update_individual(module, clustering_attempt)?;

    Ok(())
}

/// Сравнивает два процесса с помощью AI
fn compare_processes(module: &mut BusinessProcessAnalysisModule, process1_id: &str, process2_id: &str) -> Result<bool, Box<dyn std::error::Error>> {
    let mut process1 = Individual::default();
    let mut process2 = Individual::default();

    // Загружаем процессы
    if module.backend.storage.get_individual(process1_id, &mut process1) != ResultCode::Ok {
        return Err(format!("Failed to load process {}", process1_id).into());
    }
    if module.backend.storage.get_individual(process2_id, &mut process2) != ResultCode::Ok {
        return Err(format!("Failed to load process {}", process2_id).into());
    }

    process1.parse_all();
    process2.parse_all();

    // Подготавливаем данные для сравнения
    let comparison_data = prepare_comparison_data(module, &mut process1, &mut process2)?;
    let system_prompt = get_system_prompt(module, "v-bpa:ClusterizeProcessesPrompt")?;

    let parameters = prepare_comparison_parameters(module.model.clone(), system_prompt, comparison_data)?;

    // Отправляем запрос к AI
    let rt = Runtime::new()?;
    let is_similar = rt.block_on(async { send_comparison_request(module, parameters).await })?;

    Ok(is_similar)
}

/// Подготавливает данные о процессах для анализа AI
fn prepare_comparison_data(
    module: &mut BusinessProcessAnalysisModule,
    process1: &mut Individual,
    process2: &mut Individual,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    Ok(serde_json::json!({
        "process1": extract_process_json(process1, module)?,
        "process2": extract_process_json(process2, module)?
    }))
}

/// Подготавливает параметры запроса для сравнения процессов
fn prepare_comparison_parameters(
    model: String,
    system_prompt: String,
    comparison_data: serde_json::Value,
) -> Result<openai_dive::v1::resources::chat::ChatCompletionParameters, Box<dyn std::error::Error>> {
    let json_schema = serde_json::json!({
        "type": "object",
        "properties": {
            "are_similar": {
                "type": "boolean",
                "description": "Являются ли процессы похожими"
            }
        },
        "required": ["are_similar"]
    });

    let parameters = ChatCompletionParametersBuilder::default()
        .model(model)
        .messages(vec![
            ChatMessage::System {
                content: ChatMessageContent::Text(system_prompt),
                name: None,
            },
            ChatMessage::User {
                content: ChatMessageContent::Text(comparison_data.to_string()),
                name: None,
            },
        ])
        .response_format(ChatCompletionResponseFormat::JsonSchema(JsonSchemaBuilder::default().name("process_comparison").schema(json_schema).strict(true).build()?))
        .build()?;

    Ok(parameters)
}

/// Отправляет запрос к API AI и получает результат сравнения
async fn send_comparison_request(
    module: &mut BusinessProcessAnalysisModule,
    parameters: openai_dive::v1::resources::chat::ChatCompletionParameters,
) -> Result<bool, Box<dyn std::error::Error>> {
    info!("Sending comparison request to OpenAI API");
    let result = module.client.chat().create(parameters).await?;

    if let Some(choice) = result.choices.first() {
        if let ChatMessage::Assistant {
            content: Some(ChatMessageContent::Text(text)),
            ..
        } = &choice.message
        {
            let response: serde_json::Value = serde_json::from_str(text)?;
            Ok(response["are_similar"].as_bool().unwrap_or(false))
        } else {
            Err("Unexpected message format".into())
        }
    } else {
        Err("No response from AI".into())
    }
}

/// Формирует кластеры на основе найденных похожих пар
fn build_clusters(module: &mut BusinessProcessAnalysisModule, clustering_attempt: &mut Individual) -> Result<(), Box<dyn std::error::Error>> {
    // Получаем все похожие пары
    let similar_pairs = clustering_attempt.get_literals("v-bpa:similarPairs").unwrap_or_default();

    info!("Building clusters from {} similar pairs", similar_pairs.len());

    // Строим граф связей между процессами
    let mut adjacency_list: HashMap<String, HashSet<String>> = HashMap::new();

    // Заполняем граф связями
    for pair in similar_pairs {
        let parts: Vec<&str> = pair.split(',').collect();
        if parts.len() == 2 {
            // Добавляем связь в обе стороны
            adjacency_list.entry(parts[0].to_string()).or_default().insert(parts[1].to_string());
            adjacency_list.entry(parts[1].to_string()).or_default().insert(parts[0].to_string());
        }
    }

    // Находим все процессы для кластеризации
    let all_processes: HashSet<String> = clustering_attempt.get_literals("v-bpa:processesToAnalyze").unwrap_or_default().into_iter().collect();

    // Добавляем изолированные процессы в граф
    for process in &all_processes {
        adjacency_list.entry(process.clone()).or_default();
    }

    // Находим связные компоненты (кластеры)
    let clusters = find_connected_components(&adjacency_list);
    info!("Found {} clusters", clusters.len());

    // Создаем кластеры в системе
    for (cluster_index, processes) in clusters.iter().enumerate() {
        if processes.len() >= 1 {
            info!("Creating cluster {} with {} processes", cluster_index + 1, processes.len());
            match create_cluster(module, processes.iter().cloned().collect::<Vec<_>>(), clustering_attempt) {
                Ok(cluster_id) => {
                    info!("Created cluster {}", cluster_id);
                },
                Err(e) => {
                    error!("Failed to create cluster: {}", e);
                },
            }
        }
    }

    // Обновляем статус
    clustering_attempt.set_uri("v-bpa:clusterizationStatus", "v-bpa:Completed");

    update_individual(module, clustering_attempt)?;

    Ok(())
}

/// Находит связные компоненты в графе процессов
fn find_connected_components(adjacency_list: &HashMap<String, HashSet<String>>) -> Vec<HashSet<String>> {
    let mut clusters = Vec::new();
    let mut visited = HashSet::new();

    // Проходим по всем вершинам
    for node in adjacency_list.keys() {
        if !visited.contains(node) {
            // Нашли новый кластер
            let mut cluster = HashSet::new();
            let mut queue = vec![node.clone()];
            visited.insert(node.clone());
            cluster.insert(node.clone());

            // Обходим все связанные вершины
            while let Some(current) = queue.pop() {
                if let Some(neighbors) = adjacency_list.get(&current) {
                    for neighbor in neighbors {
                        if !visited.contains(neighbor) {
                            visited.insert(neighbor.clone());
                            cluster.insert(neighbor.clone());
                            queue.push(neighbor.clone());
                        }
                    }
                }
            }

            clusters.push(cluster);
        }
    }

    clusters
}

/// Создает новый кластер процессов
fn create_cluster(module: &mut BusinessProcessAnalysisModule, processes: Vec<String>, clustering_attempt: &mut Individual) -> Result<String, Box<dyn std::error::Error>> {
    let mut cluster = Individual::default();

    // Генерируем уникальный ID для кластера
    let cluster_id = format!("d:bpa_cluster_{}", uuid::Uuid::new_v4());
    cluster.set_id(&cluster_id);

    // Устанавливаем тип кластера
    cluster.set_uri("rdf:type", "v-bpa:ProcessCluster");

    // Добавляем все процессы в кластер
    cluster.set_uris("v-bpa:hasProcess", processes);

    // Сохраняем кластер
    if let Err(e) = module.backend.mstorage_api.update_or_err(&module.ticket, "BPA", "", IndvOp::Put, &mut cluster) {
        return Err(std::io::Error::new(io::ErrorKind::Other, format!("Failed to save cluster, err={:?}", e)).into());
    }

    // Добавляем ссылку на кластер в попытку кластеризации
    clustering_attempt.set_uri("v-bpa:foundClusters", &cluster_id);

    Ok(cluster_id)
}

/// Вспомогательная функция для сохранения изменений в индивиде
fn update_individual(module: &mut BusinessProcessAnalysisModule, individual: &mut Individual) -> Result<(), Box<dyn std::error::Error>> {
    if let Err(e) = module.backend.mstorage_api.update_or_err(&module.ticket, "BPA", "", IndvOp::Put, individual) {
        return Err(std::io::Error::new(std::io::ErrorKind::Other, format!("Failed to update individual, err={:?}", e)).into());
    }
    Ok(())
}
