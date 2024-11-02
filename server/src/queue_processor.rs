// queue_processor.rs

use crate::business_process_handler::analyze_process_justification;
use crate::clustering_handler::analyze_process_clusters;
use openai_dive::v1::api::Client;
use v_common::ft_xapian::xapian_reader::XapianReader;
use v_common::module::module_impl::{get_inner_binobj_as_individual, PrepareError};
use v_common::module::veda_backend::Backend;
use v_common::module::veda_module::VedaQueueModule;
use v_common::onto::individual::Individual;
use v_common::onto::parser::parse_raw;
use v_common::v_api::api_client::IndvOp;

pub struct BusinessProcessAnalysisModule {
    pub client: Client,
    pub backend: Backend,
    pub xr: XapianReader,
    pub model: String,
    pub ticket: String,
}

impl VedaQueueModule for BusinessProcessAnalysisModule {
    fn before_batch(&mut self, _size_batch: u32) -> Option<u32> {
        None
    }

    fn prepare(&mut self, queue_element: &mut Individual) -> Result<bool, PrepareError> {
        let source = queue_element.get_first_literal("src").unwrap_or_default();
        if source == "BPA" {
            return Ok(true);
        }

        let cmd = IndvOp::from_i64(queue_element.get_first_integer("cmd").unwrap_or(IndvOp::None.to_i64()));
        if cmd == IndvOp::Remove || cmd == IndvOp::None {
            return Ok(true);
        }

        // Получаем новое состояние индивидуала из элемента очереди
        let mut new_state = Individual::default();
        if !get_inner_binobj_as_individual(queue_element, "new_state", &mut new_state) {
            //error!("Failed to get 'new_state' from queue element, queue_element.id ={}", queue_element.get_id());
            return Ok(false);
        }

        // Парсим новое состояние
        if let Err(e) = parse_raw(&mut new_state) {
            error!("Failed to parse new state: {:?}", e);
            return Ok(false);
        }

        // Обработка в зависимости от типа индивида
        if new_state.any_exists("rdf:type", &[&"v-bpa:BusinessProcess".to_string()]) {
            info!("Found a saved object of type 'v-bpa:BusinessProcess' with ID: {}", new_state.get_id());

            // Анализируем обоснованность бизнес-процесса
            if let Err(e) = analyze_process_justification(self, &mut new_state) {
                error!("Error analyzing business process justification: {:?}", e);
            }
        } else if new_state.any_exists("rdf:type", &[&"v-bpa:ClusterizationAttempt".to_string()]) {
            let counter = new_state.get_first_integer("v-s:updateCounter").unwrap_or(-1);
            info!("Found a saved object of type 'v-bpa:ClusterizationAttempt' with ID: {}:{}", new_state.get_id(), counter);

            // Выполняем шаг кластеризации
            if let Err(e) = analyze_process_clusters(self, &mut new_state) {
                error!("Error analyzing process clusters: {:?}", e);
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
