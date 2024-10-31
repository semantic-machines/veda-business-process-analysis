// queue_processor.rs

use crate::business_process_handler::analyze_process_justification;
use openai_dive::v1::api::Client;
use v_common::module::module_impl::{get_inner_binobj_as_individual, PrepareError};
use v_common::module::veda_backend::Backend;
use v_common::module::veda_module::VedaQueueModule;
use v_common::onto::individual::Individual;
use v_common::onto::parser::parse_raw;
use crate::clustering_handler::analyze_process_clusters;

pub struct BusinessProcessAnalysisModule {
    pub client: Client,
    pub backend: Backend,
    pub model: String,
    pub ticket: String,
}

impl VedaQueueModule for BusinessProcessAnalysisModule {
    fn before_batch(&mut self, _size_batch: u32) -> Option<u32> {
        None
    }

    fn prepare(&mut self, queue_element: &mut Individual) -> Result<bool, PrepareError> {
        let event_id = queue_element.get_first_literal("event_id").unwrap_or_default();
        if event_id == "BPA" {
            return Ok(true);
        }

        // Парсим элемент очереди
        if let Err(e) = parse_raw(queue_element) {
            error!("Failed to parse queue element: {:?}", e);
            return Ok(false);
        }

        // Получаем новое состояние индивидуала из элемента очереди
        let mut new_state = Individual::default();
        if !get_inner_binobj_as_individual(queue_element, "new_state", &mut new_state) {
            error!("Failed to get 'new_state' from queue element");
            return Ok(false);
        }

        // Парсим новое состояние
        if let Err(e) = parse_raw(&mut new_state) {
            error!("Failed to parse new state: {:?}", e);
            return Ok(false);
        }

        // Проверяем, является ли новый индивидуал типом 'v-bpa:BusinessProcess'
        if new_state.any_exists("rdf:type", &[&"v-bpa:BusinessProcess".to_string()]) {
            info!("Found a saved object of type 'v-bpa:BusinessProcess' with ID: {}", new_state.get_id());

            // Обрабатываем бизнес-процесс
            if let Err(e) = analyze_process_justification(self, &mut new_state) {
                error!("Error processing BusinessProcess: {:?}", e);
            }
        } else if new_state.any_exists("rdf:type", &[&"v-bpa:ClusterizationAttempt".to_string()]) {
            info!("Found a saved object of type 'v-bpa:ClusterizationAttempt' with ID: {}", new_state.get_id());

            // Проверяем статус кластеризации
            let status = new_state.get_first_literal("v-bpa:clusteringStatus").unwrap_or_default();
            if status.is_empty() || status == "v-bpa:Pending" {
                // Выполняем кластеризацию только для новых или ожидающих попыток
                if let Err(e) = analyze_process_clusters(self, &mut new_state) {
                    error!("Error analyzing process clusters: {:?}", e);
                }
            }
        }

        Ok(true)
    }

    fn after_batch(&mut self, _prepared_batch_size: u32) -> Result<bool, PrepareError> {
        Ok(true)
    }

    fn heartbeat(&mut self) -> Result<(), PrepareError> {
        Ok(())
    }

    fn before_start(&mut self) {}

    fn before_exit(&mut self) {}
}
