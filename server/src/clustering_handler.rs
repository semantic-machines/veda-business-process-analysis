use crate::clustering_common;
use crate::common::{extract_process_json, get_individuals_uris_by_query, get_individuals_uris_by_type};
use crate::prompt_manager::get_system_prompt;
use crate::queue_processor::BusinessProcessAnalysisModule;
use serde_json;
use std::collections::{HashMap, HashSet};
use std::io;
use tokio::runtime::Runtime;
use v_common::onto::datatype::Lang;
use v_common::onto::individual::Individual;
use v_common::v_api::api_client::IndvOp;
use v_common::v_api::obj::ResultCode;

/// Результат сравнения пары процессов
#[derive(Debug)]
enum ComparisonResult {
    Completed, // Все пары сравнены
    Continue,  // Нужно продолжить сравнение следующей пары
}

/// Состояние процесса сравнения
#[derive(Debug)]
struct ComparisonState {
    x: usize,
    y: usize,
}

/// Обновляет временные метки для отслеживания активности процесса кластеризации
fn update_activity_timestamps(clustering_attempt: &mut Individual, status: &str) -> Result<(), Box<dyn std::error::Error>> {
    let current_time = chrono::Utc::now().timestamp();

    // Всегда обновляем время последней активности
    clustering_attempt.set_datetime("v-bpa:lastActivityAt", current_time);

    // Специфичные действия для разных статусов
    match status {
        "" => {
            // Начало работы
            clustering_attempt.set_datetime("v-bpa:startDate", current_time);
            clustering_attempt.remove("v-bpa:endDate");
        },
        "v-bpa:Completed" | "v-bpa:Failed" | "v-bpa:Cancelled" => {
            // Момент завершения работы (успешного, с ошибкой или отмены)
            clustering_attempt.set_datetime("v-bpa:endDate", current_time);
        },
        _ => (), // Для других статусов дополнительных действий не требуется
    }

    Ok(())
}

/// Проверяет наличие команд управления процессом
fn check_control_action(module: &mut BusinessProcessAnalysisModule, clustering_attempt: &mut Individual) -> Result<bool, Box<dyn std::error::Error>> {
    // Get fresh state from storage
    let mut current_state = Individual::default();
    if module.backend.storage.get_individual(clustering_attempt.get_id(), &mut current_state) != ResultCode::Ok {
        error!("Failed to load current state for attempt {}", clustering_attempt.get_id());
        return Err(format!("Failed to load current state for attempt {}", clustering_attempt.get_id()).into());
    }

    // Check control action from fresh state
    if let Some(control_action) = current_state.get_first_literal("v-bpa:controlAction") {
        match control_action.as_str() {
            "v-bpa:StopExecution" => {
                info!("Received stop command for clustering attempt {}", clustering_attempt.get_id());
                clustering_attempt.set_uri("v-bpa:hasClusterizationStatus", "v-bpa:Paused");
                clustering_attempt.set_uri("v-bpa:hasExecutionState", "v-bpa:ExecutionPaused");
                update_activity_timestamps(clustering_attempt, "v-bpa:Paused")?;
                clustering_common::update_individual(module, clustering_attempt, IndvOp::SetIn)?;
                return Ok(false);
            },
            "v-bpa:CancelExecution" => {
                info!("Received cancel command for clustering attempt {}", clustering_attempt.get_id());
                clustering_attempt.set_uri("v-bpa:hasClusterizationStatus", "v-bpa:Cancelled");
                clustering_attempt.set_uri("v-bpa:hasExecutionState", "v-bpa:ExecutionTerminated");
                update_activity_timestamps(clustering_attempt, "v-bpa:Cancelled")?;
                clustering_common::update_individual(module, clustering_attempt, IndvOp::SetIn)?;
                return Ok(false);
            },
            _ => (),
        }
        // After processing, remove the control action from our instance
        clustering_attempt.set_uri("v-bpa:controlAction", "v-bpa:NoActionExecution");
    }
    Ok(true)
}

/// Анализирует бизнес-процессы и определяет кластеры схожих процессов
///
/// # Алгоритм работы
/// 1. Инициализация: загрузка процессов и подготовка состояния
/// 2. Сравнение всех пар процессов на предмет схожести
/// 3. Формирование связных групп (кластеров) на основе схожих пар
/// 4. Создание и сохранение кластеров в базе
///
/// # Управление процессом
/// - Возможна остановка/возобновление через controlAction
/// - Ведется учет затраченного времени
/// - Обновляется процент выполнения
pub fn analyze_process_clusters(module: &mut BusinessProcessAnalysisModule, clustering_attempt: &mut Individual) -> Result<(), Box<dyn std::error::Error>> {
    let ca = get_individuals_uris_by_query(module, &format!("'rdf:type' == 'v-bpa:ClusterizationAttempt' && 'v-bpa:hasExecutionState' == 'v-bpa:ExecutionInProgress' && '@' != '{}'", clustering_attempt.get_id()))?;
    if !ca.is_empty() {
        let error_msg = "Невозможно начать расчет - уже существует активный процесс кластеризации";
        clustering_attempt.set_string("v-bpa:lastError", error_msg, Lang::none());
        clustering_attempt.set_uri("v-bpa:hasClusterizationStatus", "v-bpa:Failed");
        clustering_attempt.set_uri("v-bpa:hasExecutionState", "v-bpa:ExecutionError");
        update_activity_timestamps(clustering_attempt, "v-bpa:Failed")?;
        clustering_common::update_individual(module, clustering_attempt, IndvOp::SetIn)?;
        return Err(error_msg.into());
    }

    let mut comparison_state = None;

    if !clustering_attempt.any_exists("v-bpa:controlAction", &["v-bpa:StartExecution", "v-bpa:ResumeExecution"]) {
        return Ok(());
    }

    info!("Starting process cluster analysis for attempt: {}", clustering_attempt.get_id());

    loop {
        // Проверяем команды управления процессом
        if !check_control_action(module, clustering_attempt)? {
            return Ok(());
        }

        // Получаем актуальный статус на каждой итерации
        let status = clustering_attempt.get_first_literal("v-bpa:hasClusterizationStatus").unwrap_or_default();

        match status.as_str() {
            "" => {
                info!("Starting new clustering attempt: {}", clustering_attempt.get_id());
                match initialize_clustering(module, clustering_attempt) {
                    Ok(_) => {
                        update_activity_timestamps(clustering_attempt, "")?;
                        clustering_attempt.set_uri("v-bpa:hasExecutionState", "v-bpa:ExecutionInProgress");
                        clustering_common::update_individual(module, clustering_attempt, IndvOp::SetIn)?;
                        comparison_state = Some(ComparisonState {
                            x: 0,
                            y: 1,
                        });
                    },
                    Err(e) => handle_error(module, clustering_attempt, e)?,
                }
            },
            "v-bpa:Paused" => {
                if let Some(control_action) = clustering_attempt.get_first_literal("v-bpa:controlAction") {
                    if control_action == "v-bpa:ResumeExecution" {
                        info!("Resuming clustering attempt {}", clustering_attempt.get_id());
                        clustering_attempt.set_uri("v-bpa:controlAction", "v-bpa:NoActionExecution");

                        clustering_attempt.set_uri("v-bpa:hasClusterizationStatus", "v-bpa:ComparingPairs");
                        clustering_attempt.set_uri("v-bpa:hasExecutionState", "v-bpa:ExecutionInProgress");
                        clustering_common::update_individual(module, clustering_attempt, IndvOp::SetIn)?;
                    } else if control_action == "v-bpa:CancelExecution" {
                        info!("Cancelling paused clustering attempt {}", clustering_attempt.get_id());
                        clustering_attempt.set_uri("v-bpa:hasClusterizationStatus", "v-bpa:Cancelled");
                        clustering_attempt.set_uri("v-bpa:hasExecutionState", "v-bpa:ExecutionTerminated");
                        update_activity_timestamps(clustering_attempt, "v-bpa:Cancelled")?;
                        clustering_attempt.set_uri("v-bpa:controlAction", "v-bpa:NoActionExecution");

                        clustering_common::update_individual(module, clustering_attempt, IndvOp::SetIn)?;
                        return Ok(());
                    }
                }
                continue;
            },
            "v-bpa:ComparingPairs" => {
                if comparison_state.is_none() {
                    let current_pair =
                        clustering_attempt.get_literals("v-bpa:currentPairIndex").and_then(|pairs| pairs.first().cloned()).ok_or("No current pair index found")?;

                    let indices: Vec<usize> = current_pair.split(',').map(|s| s.parse::<usize>()).collect::<Result<Vec<_>, _>>()?;

                    comparison_state = Some(ComparisonState {
                        x: indices[0],
                        y: indices[1],
                    });
                }

                match compare_next_pair(module, clustering_attempt, comparison_state.as_mut().unwrap()) {
                    Ok(ComparisonResult::Completed) => {
                        clustering_attempt.set_uri("v-bpa:hasClusterizationStatus", "v-bpa:PairsCompared");
                        update_activity_timestamps(clustering_attempt, "v-bpa:PairsCompared")?;
                        clustering_common::update_individual(module, clustering_attempt, IndvOp::SetIn)?;
                    },
                    Ok(ComparisonResult::Continue) => {
                        update_activity_timestamps(clustering_attempt, "v-bpa:ComparingPairs")?;
                    },
                    Err(e) => handle_error(module, clustering_attempt, e)?,
                }
            },
            "v-bpa:PairsCompared" => {
                info!("Building clusters for attempt: {}", clustering_attempt.get_id());
                match build_clusters(module, clustering_attempt) {
                    Ok(_) => {
                        clustering_attempt.set_uri("v-bpa:hasClusterizationStatus", "v-bpa:Completed");
                        clustering_attempt.set_uri("v-bpa:hasExecutionState", "v-bpa:ExecutionCompleted");
                        update_activity_timestamps(clustering_attempt, "v-bpa:Completed")?;
                        clustering_attempt.set_uri("v-bpa:controlAction", "v-bpa:NoActionExecution");

                        clustering_common::update_individual(module, clustering_attempt, IndvOp::SetIn)?;
                    },
                    Err(e) => handle_error(module, clustering_attempt, e)?,
                }
            },
            "v-bpa:Completed" => {
                info!("Clustering already completed for attempt: {}", clustering_attempt.get_id());
                break;
            },
            "v-bpa:Failed" => {
                info!("Clustering attempt {} failed", clustering_attempt.get_id());
                let error_msg = clustering_attempt.get_first_literal("v-bpa:lastError").unwrap_or("Unknown error".to_string());
                return Err(error_msg.into());
            },
            "v-bpa:Cancelled" => {
                info!("Clustering attempt {} was cancelled", clustering_attempt.get_id());
                return Ok(());
            },
            _ => {
                error!("Unknown clustering status: {}", status);
                clustering_attempt.set_string("v-bpa:lastError", "Invalid clustering status", Lang::none());
                clustering_attempt.set_uri("v-bpa:hasClusterizationStatus", "v-bpa:Failed");
                clustering_attempt.set_uri("v-bpa:hasExecutionState", "v-bpa:ExecutionError");
                clustering_attempt.set_uri("v-bpa:controlAction", "v-bpa:NoActionExecution");

                update_activity_timestamps(clustering_attempt, "v-bpa:Failed")?;
                clustering_common::update_individual(module, clustering_attempt, IndvOp::SetIn)?;
                return Err("Invalid clustering status".into());
            },
        }
    }

    info!("Process cluster analysis completed for attempt: {}", clustering_attempt.get_id());
    Ok(())
}

/// Вычисляет прогресс кластеризации и оставшееся время
fn calculate_clustering_metrics(state: &ComparisonState, total_processes: usize) -> (i64, i64) {
    // Вычисляем общее количество пар для сравнения
    let total_pairs = (total_processes * (total_processes - 1)) / 2;

    // Вычисляем количество уже сравненных пар
    let mut completed_pairs = 0;
    for x in 0..state.x {
        completed_pairs += total_processes - (x + 1);
    }
    completed_pairs += state.y - state.x - 1;

    // Вычисляем прогресс в процентах
    let progress = ((completed_pairs as f64 / total_pairs as f64) * 100.0) as i64;

    // Оцениваем оставшееся время (считаем, что одно сравнение занимает примерно 5 секунд)
    let seconds_per_comparison = 5;
    let remaining_pairs = total_pairs - completed_pairs;
    let estimated_time = (remaining_pairs * seconds_per_comparison) as i64;

    (progress, estimated_time)
}

/// Инициализирует процесс кластеризации
/// - Загружает все бизнес-процессы
/// - Подготавливает состояние для сравнения
/// - Устанавливает начальные значения прогресса
fn initialize_clustering(module: &mut BusinessProcessAnalysisModule, clustering_attempt: &mut Individual) -> Result<(), Box<dyn std::error::Error>> {
    info!("Initializing clustering process");

    let process_ids = get_individuals_uris_by_type(module, "v-bpa:BusinessProcess")?;

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
    clustering_attempt.set_uri("v-bpa:hasClusterizationStatus", "v-bpa:ComparingPairs");
    clustering_attempt.remove("v-bpa:similarPairs");
    clustering_attempt.remove("v-bpa:controlAction");

    // Инициализируем начальные значения прогресса и времени
    clustering_attempt.set_integer("v-bpa:clusterizationProgress", 0);
    clustering_attempt.set_integer("v-bpa:estimatedTime", ((process_len * (process_len - 1)) / 2 * 5) as i64);

    clustering_common::update_individual(module, clustering_attempt, IndvOp::Put)?;
    info!("Successfully initialized clustering attempt {} with {} processes", clustering_attempt.get_id(), process_len);

    Ok(())
}

/// Сравнивает следующую пару процессов и обновляет состояние
/// Возвращает:
/// - Completed если все пары сравнены
/// - Continue если есть еще пары для сравнения
/// - Ошибку при проблемах сравнения
fn compare_next_pair(
    module: &mut BusinessProcessAnalysisModule,
    clustering_attempt: &mut Individual,
    state: &mut ComparisonState,
) -> Result<ComparisonResult, Box<dyn std::error::Error>> {
    let processes = clustering_attempt.get_literals("v-bpa:processesToAnalyze").ok_or("No processes to analyze found")?;

    if state.x >= processes.len() || state.y >= processes.len() {
        info!("All process pairs have been compared");
        return Ok(ComparisonResult::Completed);
    }

    // Сравниваем текущую пару
    let is_similar = compare_processes(module, &processes[state.x], &processes[state.y])?;
    info!(
        "Comparison result for processes {} and {}: {}",
        processes[state.x],
        processes[state.y],
        if is_similar {
            "similar"
        } else {
            "different"
        }
    );

    if is_similar {
        // Сохраняем похожую пару
        let pair = format!("{},{}", processes[state.x], processes[state.y]);
        clustering_attempt.add_string("v-bpa:similarPairs", &pair, Lang::none());
    }

    // Вычисляем и сохраняем следующую пару
    let old_x = state.x;
    if state.y + 1 < processes.len() {
        state.y += 1;
    } else {
        state.x += 1;
        state.y = state.x + 1;
    }

    // Сохраняем состояние в базу только если нашли похожие процессы или изменился x
    if is_similar || state.x != old_x {
        // Вычисляем метрики кластеризации
        let (progress, estimated_time) = calculate_clustering_metrics(state, processes.len());

        // Обновляем все поля
        clustering_attempt.set_string("v-bpa:currentPairIndex", &format!("{},{}", state.x, state.y), Lang::none());
        clustering_attempt.set_integer("v-bpa:clusterizationProgress", progress);
        clustering_attempt.set_integer("v-bpa:estimatedTime", estimated_time);

        info!("Updating clustering metrics - Progress: {:.1}%, Estimated time remaining: {} seconds", progress, estimated_time);

        clustering_common::update_individual(module, clustering_attempt, IndvOp::SetIn)?;
    }

    Ok(ComparisonResult::Continue)
}
/// Сравнивает два процесса с помощью AI
fn compare_processes(module: &mut BusinessProcessAnalysisModule, process1_id: &str, process2_id: &str) -> Result<bool, Box<dyn std::error::Error>> {
    let mut process1 = Individual::default();
    let mut process2 = Individual::default();

    // Загружаем процессы
    if module.backend.storage.get_individual(process1_id, &mut process1) != ResultCode::Ok {
        error!("Failed to load process {}", process1_id);
        return Err(format!("Failed to load process {}", process1_id).into());
    }
    if module.backend.storage.get_individual(process2_id, &mut process2) != ResultCode::Ok {
        error!("Failed to load process {}", process2_id);
        return Err(format!("Failed to load process {}", process2_id).into());
    }

    process1.parse_all();
    process2.parse_all();

    // Подготавливаем данные для сравнения
    let comparison_data = prepare_comparison_data(module, &mut process1, &mut process2)?;
    let system_prompt = get_system_prompt(module, "v-bpa:ClusterizeProcessesPrompt")?;

    let parameters = clustering_common::prepare_comparison_parameters(module.model.clone(), system_prompt, comparison_data)?;

    // Отправляем запрос к AI
    let rt = Runtime::new()?;
    let is_similar = rt.block_on(async { clustering_common::send_comparison_request(module, parameters).await })?;

    Ok(is_similar)
}

/// Подготавливает данные о процессах для анализа AI
fn prepare_comparison_data(
    module: &mut BusinessProcessAnalysisModule,
    process1: &mut Individual,
    process2: &mut Individual,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let result = serde_json::json!({
        "process1": extract_process_json(process1, module)?,
        "process2": extract_process_json(process2, module)?
    });

    Ok(result)
}

/// Формирует кластеры на основе найденных похожих пар процессов
fn build_clusters(module: &mut BusinessProcessAnalysisModule, clustering_attempt: &mut Individual) -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting cluster building process");

    let similar_pairs = clustering_attempt.get_literals("v-bpa:similarPairs").unwrap_or_default();
    info!("Found {} similar pairs to process", similar_pairs.len());

    if similar_pairs.is_empty() {
        info!("No similar pairs found, skipping cluster creation");
        clustering_attempt.set_uri("v-bpa:hasClusterizationStatus", "v-bpa:Completed");
        clustering_common::update_individual(module, clustering_attempt, IndvOp::SetIn)?;
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
            info!("Processing cluster {} with {} processes", cluster_index + 1, processes.len());
            match create_cluster(module, processes.iter().cloned().collect::<Vec<_>>(), clustering_attempt) {
                Ok(cluster_id) => {
                    info!("Successfully created cluster {} with {} processes", cluster_id, processes.len());
                    created_clusters += 1;
                },
                Err(e) => {
                    error!("Failed to create cluster {}: {}", cluster_index + 1, e);
                    clustering_attempt.set_string("v-bpa:lastError", &e.to_string(), Lang::none());
                    clustering_attempt.set_uri("v-bpa:hasClusterizationStatus", "v-bpa:Failed");
                    clustering_attempt.set_uri("v-bpa:hasExecutionState", "v-bpa:ExecutionError");
                    update_activity_timestamps(clustering_attempt, "v-bpa:Failed")?;
                    clustering_common::update_individual(module, clustering_attempt, IndvOp::SetIn)?;
                    return Err(e);
                },
            }
        } else {
            info!("Skipping cluster {} as it contains only {} process", cluster_index + 1, processes.len());
        }
    }

    info!("Created {} clusters from {} potential groups", created_clusters, clusters.len());
    Ok(())
}

/// Находит связные компоненты в графе процессов с помощью поиска в ширину
///
/// # Алгоритм
/// 1. Начинаем с непосещенной вершины
/// 2. Запускаем поиск в ширину из этой вершины
/// 3. Все достижимые вершины формируют один кластер
/// 4. Повторяем для оставшихся непосещенных вершин
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

/// Создает новый кластер процессов в базе
fn create_cluster(module: &mut BusinessProcessAnalysisModule, processes: Vec<String>, clustering_attempt: &mut Individual) -> Result<String, Box<dyn std::error::Error>> {
    info!("Creating new cluster for {} processes", processes.len());
    let mut cluster = Individual::default();

    // Генерируем уникальный ID для кластера
    let cluster_id = format!("d:bpa_cluster_{}", uuid::Uuid::new_v4());
    cluster.set_id(&cluster_id);
    cluster.set_uri("rdf:type", "v-bpa:ProcessCluster");
    cluster.set_uris("v-bpa:hasProcess", processes);

    if let Err(e) = module.backend.mstorage_api.update_or_err(&module.ticket, "", "BPA", IndvOp::Put, &mut cluster) {
        error!("Failed to save cluster {}: {:?}", cluster_id, e);
        return Err(std::io::Error::new(io::ErrorKind::Other, format!("Failed to save cluster, err={:?}", e)).into());
    }

    info!("Adding cluster reference to clustering attempt");
    clustering_attempt.add_uri("v-bpa:foundClusters", &cluster_id);

    Ok(cluster_id)
}

/// Обработчик ошибок процесса кластеризации
fn handle_error(
    module: &mut BusinessProcessAnalysisModule,
    clustering_attempt: &mut Individual,
    error: Box<dyn std::error::Error>,
) -> Result<(), Box<dyn std::error::Error>> {
    error!("Error in clustering process: {}", error);
    clustering_attempt.set_string("v-bpa:lastError", &error.to_string(), Lang::none());
    clustering_attempt.set_uri("v-bpa:hasClusterizationStatus", "v-bpa:Failed");
    clustering_attempt.set_uri("v-bpa:hasExecutionState", "v-bpa:ExecutionError");
    update_activity_timestamps(clustering_attempt, "v-bpa:Failed")?;
    clustering_common::update_individual(module, clustering_attempt, IndvOp::SetIn)?;
    Err(error)
}
