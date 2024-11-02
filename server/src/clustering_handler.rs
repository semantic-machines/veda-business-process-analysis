use crate::common::extract_process_json;
use crate::prompt_manager::get_system_prompt;
use crate::queue_processor::BusinessProcessAnalysisModule;
use openai_dive::v1::resources::chat::{ChatCompletionParametersBuilder, ChatCompletionResponseFormat, ChatMessage, ChatMessageContent, JsonSchemaBuilder};
use serde_json;
use std::collections::{HashMap, HashSet};
use std::{io, thread, time};
use tokio::runtime::Runtime;
use v_common::onto::datatype::Lang;
use v_common::onto::individual::Individual;
use v_common::search::common::{FTQuery, QueryResult};
use v_common::v_api::api_client::IndvOp;
use v_common::v_api::obj::ResultCode;

/// Результат сравнения пары процессов
#[derive(Debug)]
enum ComparisonResult {
    Completed, // Все пары сравнены
    Continue,  // Нужно продолжить сравнение следующей пары
}

/// Анализирует бизнес-процессы и определяет кластеры схожих процессов
pub fn analyze_process_clusters(module: &mut BusinessProcessAnalysisModule, clustering_attempt: &mut Individual) -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting process cluster analysis for attempt: {}", clustering_attempt.get_id());

    loop {
        // Получаем актуальный статус на каждой итерации
        let status = clustering_attempt.get_literals("v-bpa:clusterizationStatus").and_then(|s| s.first().cloned()).unwrap_or_default();

        //info!("Current clustering status: {}", if status.is_empty() { "New" } else { &status });

        match status.as_str() {
            "" => {
                info!("Starting new clustering attempt: {}", clustering_attempt.get_id());
                initialize_clustering(module, clustering_attempt)?;
            },
            "v-bpa:ComparingPairs" => {
                //info!("Continuing pair comparison for attempt: {}", clustering_attempt.get_id());
                match compare_next_pair(module, clustering_attempt) {
                    Ok(ComparisonResult::Completed) => {
                        clustering_attempt.set_uri("v-bpa:clusterizationStatus", "v-bpa:PairsCompared");
                        update_individual(module, clustering_attempt)?;
                    },
                    Ok(ComparisonResult::Continue) => {
                        // Просто продолжаем цикл со следующей парой
                    },
                    Err(e) => {
                        error!("Error during pair comparison: {}", e);
                        save_clustering_state(module, clustering_attempt, Some(e.to_string()))?;
                        return Err(e);
                    },
                }
            },
            "v-bpa:PairsCompared" => {
                info!("Building clusters for attempt: {}", clustering_attempt.get_id());
                build_clusters(module, clustering_attempt)?;
                clustering_attempt.set_uri("v-bpa:clusterizationStatus", "v-bpa:Completed");
                update_individual(module, clustering_attempt)?;
            },
            "v-bpa:Completed" => {
                info!("Clustering already completed for attempt: {}", clustering_attempt.get_id());
                break;
            },
            _ => {
                error!("Unknown clustering status: {}", status);
                return Err("Invalid clustering status".into());
            },
        }
    }

    info!("Process cluster analysis completed for attempt: {}", clustering_attempt.get_id());
    Ok(())
}

/// Находит все бизнес-процессы в системе
fn find_all_business_processes(module: &mut BusinessProcessAnalysisModule) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    info!("Starting business process search");
    let mut res = QueryResult::default();
    res.result_code = ResultCode::NotReady;

    let mut retry_count = 0;
    while res.result_code == ResultCode::NotReady || res.result_code == ResultCode::DatabaseModifiedError {
        info!("Attempting to query business processes (attempt {})", retry_count + 1);
        res = module.xr.query(FTQuery::new_with_user("cfg:VedaSystem", "'rdf:type' === 'v-bpa:BusinessProcess'"), &mut module.backend.storage);

        if res.result_code == ResultCode::InternalServerError {
            error!("Search failed with internal server error");
            return Err(io::Error::new(io::ErrorKind::Other, format!("Search failed with error: {:?}", res.result_code)).into());
        }

        if res.result_code != ResultCode::Ok {
            warn!("Failed to search business processes, retry in 3 seconds... (attempt {})", retry_count + 1);
            thread::sleep(time::Duration::from_secs(3));
        }
        retry_count += 1;
    }

    let mut process_ids = Vec::new();
    if res.result_code == ResultCode::Ok && res.count > 0 {
        process_ids.extend(res.result);
        info!("Successfully found {} business processes", process_ids.len());
    } else {
        info!("No business processes found in the system");
    }

    Ok(process_ids)
}

/// Инициализирует процесс кластеризации
fn initialize_clustering(module: &mut BusinessProcessAnalysisModule, clustering_attempt: &mut Individual) -> Result<(), Box<dyn std::error::Error>> {
    info!("Initializing clustering process");

    let process_ids = find_all_business_processes(module)?;

    if process_ids.is_empty() {
        error!("Clustering initialization failed: no business processes found");
        return Err("No business processes found for clustering".into());
    }

    info!("Saving {} processes for analysis", process_ids.len());
    let process_len = process_ids.len();
    clustering_attempt.set_uris("v-bpa:processesToAnalyze", process_ids);

    info!("Initializing comparison progress with first pair (0,1)");
    clustering_attempt.set_string("v-bpa:currentPairIndex", "0,1", Lang::none());

    info!("Setting initial clustering status to ComparingPairs");
    clustering_attempt.set_uri("v-bpa:clusterizationStatus", "v-bpa:ComparingPairs");
    clustering_attempt.remove("v-bpa:similarPairs");

    update_individual(module, clustering_attempt)?;
    info!("Successfully initialized clustering attempt {} with {} processes", clustering_attempt.get_id(), process_len);

    Ok(())
}

/// Сравнивает следующую пару процессов
fn compare_next_pair(module: &mut BusinessProcessAnalysisModule, clustering_attempt: &mut Individual) -> Result<ComparisonResult, Box<dyn std::error::Error>> {
    //info!("Starting comparison of next process pair");

    let current_pair = if let Some(pairs) = clustering_attempt.get_literals("v-bpa:currentPairIndex") {
        pairs.first().cloned().ok_or("No current pair index found")?
    } else {
        error!("Failed to find current pair index");
        return Err("No current pair index found".into());
    };

    let indices: Vec<usize> = current_pair.split(',').map(|s| s.parse::<usize>()).collect::<Result<Vec<_>, _>>()?;
    //info!("Current comparison indices: [{}, {}]", indices[0], indices[1]);

    let processes = clustering_attempt.get_literals("v-bpa:processesToAnalyze").ok_or("No processes to analyze found")?;

    if indices[0] >= processes.len() || indices[1] >= processes.len() {
        info!("All process pairs have been compared");
        return Ok(ComparisonResult::Completed);
    }

    //info!("Comparing processes {} and {}", processes[indices[0]], processes[indices[1]]);

    // Сравниваем текущую пару
    let is_similar = compare_processes(module, &processes[indices[0]], &processes[indices[1]])?;
    info!(
        "Comparison result for processes {} and {}: {}",
        processes[indices[0]],
        processes[indices[1]],
        if is_similar {
            "similar"
        } else {
            "different"
        }
    );

    if is_similar {
        // Сохраняем похожую пару
        let pair = format!("{},{}", processes[indices[0]], processes[indices[1]]);
        clustering_attempt.add_string("v-bpa:similarPairs", &pair, Lang::none());
        info!("Recorded similar pair: {}", pair);
    }

    // Вычисляем следующую пару
    let (next_i, next_j) = if indices[1] + 1 < processes.len() {
        (indices[0], indices[1] + 1)
    } else {
        (indices[0] + 1, indices[0] + 2)
    };

    // Обновляем индексы
    //info!("Setting next pair indices to [{}, {}]", next_i, next_j);
    clustering_attempt.set_string("v-bpa:currentPairIndex", &format!("{},{}", next_i, next_j), Lang::none());

    update_individual(module, clustering_attempt)?;
    //info!("Successfully completed comparison of current pair");

    Ok(ComparisonResult::Continue)
}

/// Сравнивает два процесса с помощью AI
fn compare_processes(module: &mut BusinessProcessAnalysisModule, process1_id: &str, process2_id: &str) -> Result<bool, Box<dyn std::error::Error>> {
    //info!("Starting detailed comparison of processes {} and {}", process1_id, process2_id);

    let mut process1 = Individual::default();
    let mut process2 = Individual::default();

    // Загружаем процессы
    //info!("Loading process data from storage");
    if module.backend.storage.get_individual(process1_id, &mut process1) != ResultCode::Ok {
        error!("Failed to load process {}", process1_id);
        return Err(format!("Failed to load process {}", process1_id).into());
    }
    if module.backend.storage.get_individual(process2_id, &mut process2) != ResultCode::Ok {
        error!("Failed to load process {}", process2_id);
        return Err(format!("Failed to load process {}", process2_id).into());
    }

    //info!("Parsing process data");
    process1.parse_all();
    process2.parse_all();

    // Подготавливаем данные для сравнения
    //info!("Preparing comparison data for AI analysis");
    let comparison_data = prepare_comparison_data(module, &mut process1, &mut process2)?;
    let system_prompt = get_system_prompt(module, "v-bpa:ClusterizeProcessesPrompt")?;

    //info!("Preparing AI request parameters");
    let parameters = prepare_comparison_parameters(module.model.clone(), system_prompt, comparison_data)?;

    // Отправляем запрос к AI
    //info!("Sending comparison request to AI service");
    let rt = Runtime::new()?;
    let is_similar = rt.block_on(async { send_comparison_request(module, parameters).await })?;

    //info!("AI comparison complete. Similarity result: {}", is_similar);
    Ok(is_similar)
}

/// Подготавливает данные о процессах для анализа AI
fn prepare_comparison_data(
    module: &mut BusinessProcessAnalysisModule,
    process1: &mut Individual,
    process2: &mut Individual,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    //info!("Preparing comparison data for processes {} and {}", process1.get_id(), process2.get_id());

    let result = serde_json::json!({
        "process1": extract_process_json(process1, module)?,
        "process2": extract_process_json(process2, module)?
    });

    //info!("Successfully prepared comparison data");
    Ok(result)
}

/// Подготавливает параметры запроса для сравнения процессов
fn prepare_comparison_parameters(
    model: String,
    system_prompt: String,
    comparison_data: serde_json::Value,
) -> Result<openai_dive::v1::resources::chat::ChatCompletionParameters, Box<dyn std::error::Error>> {
    //info!("Preparing AI comparison parameters");

    let json_schema = serde_json::json!({
        "type": "object",
        "properties": {
            "are_similar": {
                "type": "boolean",
                "description": "Являются ли процессы похожими"
            }
        },
        "required": ["are_similar"],
        "additionalProperties": false
    });

    //info!("@1 comparison_data = {}", comparison_data.to_string());

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

    //info!("Successfully prepared AI comparison parameters");
    Ok(parameters)
}

/// Отправляет запрос к API AI и получает результат сравнения
async fn send_comparison_request(
    module: &mut BusinessProcessAnalysisModule,
    parameters: openai_dive::v1::resources::chat::ChatCompletionParameters,
) -> Result<bool, Box<dyn std::error::Error>> {
    //info!("Sending comparison request to OpenAI API");
    let result = module.client.chat().create(parameters).await?;

    if let Some(choice) = result.choices.first() {
        if let ChatMessage::Assistant {
            content: Some(ChatMessageContent::Text(text)),
            ..
        } = &choice.message
        {
            let response: serde_json::Value = serde_json::from_str(text)?;
            let similarity = response["are_similar"].as_bool().unwrap_or(false);
            //info!("Received AI response. Similarity result: {}", similarity);
            Ok(similarity)
        } else {
            error!("Unexpected message format in AI response");
            Err("Unexpected message format".into())
        }
    } else {
        error!("No response received from AI");
        Err("No response from AI".into())
    }
}

/// Формирует кластеры на основе найденных похожих пар
fn build_clusters(module: &mut BusinessProcessAnalysisModule, clustering_attempt: &mut Individual) -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting cluster building process");

    let similar_pairs = clustering_attempt.get_literals("v-bpa:similarPairs").unwrap_or_default();
    info!("Found {} similar pairs to process", similar_pairs.len());

    if similar_pairs.is_empty() {
        info!("No similar pairs found, skipping cluster creation");
        clustering_attempt.set_uri("v-bpa:clusterizationStatus", "v-bpa:Completed");
        update_individual(module, clustering_attempt)?;
        return Ok(());
    }

    // Строим граф связей между процессами
    let mut adjacency_list: HashMap<String, HashSet<String>> = HashMap::new();

    info!("Building process relationship graph");

    // Добавляем только связанные процессы
    for pair in similar_pairs {
        let parts: Vec<&str> = pair.split(',').collect();
        if parts.len() == 2 {
            info!("Adding bidirectional connection between {} and {}", parts[0], parts[1]);
            adjacency_list.entry(parts[0].to_string()).or_default().insert(parts[1].to_string());
            adjacency_list.entry(parts[1].to_string()).or_default().insert(parts[0].to_string());
        }
    }

    // Находим связные компоненты (кластеры)
    let clusters = find_connected_components(&adjacency_list);
    info!("Found {} potential clusters", clusters.len());

    // Очищаем предыдущие кластеры
    clustering_attempt.remove("v-bpa:foundClusters");

    // Создаем кластеры только для групп из двух и более процессов
    let mut created_clusters = 0;
    for (cluster_index, processes) in clusters.iter().enumerate() {
        if processes.len() >= 2 {
            // Создаем кластер только если в нем 2 или больше процессов
            info!("Processing cluster {} with {} processes", cluster_index + 1, processes.len());
            match create_cluster(module, processes.iter().cloned().collect::<Vec<_>>(), clustering_attempt) {
                Ok(cluster_id) => {
                    info!("Successfully created cluster {} with {} processes", cluster_id, processes.len());
                    created_clusters += 1;
                },
                Err(e) => {
                    error!("Failed to create cluster {}: {}", cluster_index + 1, e);
                    save_clustering_state(module, clustering_attempt, Some(format!("Failed to create cluster {}: {}", cluster_index + 1, e)))?;
                    return Err(e);
                },
            }
        } else {
            info!("Skipping cluster {} as it contains only {} process", cluster_index + 1, processes.len());
        }
    }

    info!("Created {} clusters from {} potential groups", created_clusters, clusters.len());
    clustering_attempt.set_uri("v-bpa:clusterizationStatus", "v-bpa:Completed");
    update_individual(module, clustering_attempt)?;

    info!("Successfully completed cluster building process");
    Ok(())
}

/// Находит связные компоненты в графе процессов
fn find_connected_components(adjacency_list: &HashMap<String, HashSet<String>>) -> Vec<HashSet<String>> {
    info!("Starting connected components search in process graph");
    let mut clusters = Vec::new();
    let mut visited = HashSet::new();

    // Проходим по всем вершинам
    for node in adjacency_list.keys() {
        if !visited.contains(node) {
            info!("Found new unvisited node: {}", node);
            let mut cluster = HashSet::new();
            let mut queue = vec![node.clone()];
            visited.insert(node.clone());
            cluster.insert(node.clone());

            info!("Starting breadth-first search from node {}", node);
            while let Some(current) = queue.pop() {
                if let Some(neighbors) = adjacency_list.get(&current) {
                    info!("Processing {} neighbors for node {}", neighbors.len(), current);
                    for neighbor in neighbors {
                        if !visited.contains(neighbor) {
                            info!("Adding new node {} to cluster", neighbor);
                            visited.insert(neighbor.clone());
                            cluster.insert(neighbor.clone());
                            queue.push(neighbor.clone());
                        }
                    }
                }
            }

            info!("Completed cluster with {} processes", cluster.len());
            clusters.push(cluster);
        }
    }

    info!("Found {} connected components in total", clusters.len());
    clusters
}

/// Создает новый кластер процессов
fn create_cluster(module: &mut BusinessProcessAnalysisModule, processes: Vec<String>, clustering_attempt: &mut Individual) -> Result<String, Box<dyn std::error::Error>> {
    info!("Creating new cluster for {} processes", processes.len());
    let mut cluster = Individual::default();

    // Генерируем уникальный ID для кластера
    let cluster_id = format!("d:bpa_cluster_{}", uuid::Uuid::new_v4());
    //info!("Generated new cluster ID: {}", cluster_id);
    cluster.set_id(&cluster_id);

    //info!("Setting cluster type and properties");
    cluster.set_uri("rdf:type", "v-bpa:ProcessCluster");

    // Добавляем все процессы в кластер
    cluster.set_uris("v-bpa:hasProcess", processes);

    //info!("Saving cluster to storage");
    if let Err(e) = module.backend.mstorage_api.update_or_err(&module.ticket, "", "BPA", IndvOp::Put, &mut cluster) {
        error!("Failed to save cluster {}: {:?}", cluster_id, e);
        return Err(std::io::Error::new(io::ErrorKind::Other, format!("Failed to save cluster, err={:?}", e)).into());
    }

    info!("Adding cluster reference to clustering attempt");
    clustering_attempt.add_uri("v-bpa:foundClusters", &cluster_id);

    //info!("Successfully created cluster {}", cluster_id);
    Ok(cluster_id)
}

/// Вспомогательная функция для сохранения изменений в индивиде
fn update_individual(module: &mut BusinessProcessAnalysisModule, individual: &mut Individual) -> Result<(), Box<dyn std::error::Error>> {
    //info!("Updating individual {}", individual.get_id());
    if let Err(e) = module.backend.mstorage_api.update_or_err(&module.ticket, "", "BPA", IndvOp::Put, individual) {
        error!("Failed to update individual {}: {:?}", individual.get_id(), e);
        return Err(std::io::Error::new(std::io::ErrorKind::Other, format!("Failed to update individual, err={:?}", e)).into());
    }
    //info!("Successfully updated individual {}", individual.get_id());
    Ok(())
}

/// Вспомогательная функция для сохранения состояния кластеризации
fn save_clustering_state(
    module: &mut BusinessProcessAnalysisModule,
    clustering_attempt: &mut Individual,
    error_msg: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(error) = error_msg {
        clustering_attempt.set_string("v-bpa:lastError", &error, Lang::none());
        error!("Saving error state for clustering attempt: {}", error);
    }

    update_individual(module, clustering_attempt)?;
    Ok(())
}
