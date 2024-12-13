// queue_processor.rs

use crate::business_process_handler::analyze_process_justification;
use crate::cluster_optimizer::analyze_and_optimize_cluster;
use crate::clustering_handler::analyze_process_clusters;
use crate::document_status_handler::handle_document_status;
use crate::generic_processing_handler::process_generic_request;
use crate::pipeline::business_process_extraction::business_process_extraction_pipeline;
use crate::pipeline::raw_document_extracting_and_structuring::raw_document_extracting_and_structuring;
use openai_dive::v1::api::Client;
use v_common::ft_xapian::xapian_reader::XapianReader;
use v_common::module::info::ModuleInfo;
use v_common::module::module_impl::{get_inner_binobj_as_individual, PrepareError};
use v_common::module::veda_backend::Backend;
use v_common::module::veda_module::VedaQueueModule;
use v_common::onto::individual::Individual;
use v_common::onto::parser::parse_raw;
use v_common::v_api::api_client::IndvOp;

pub struct BusinessProcessAnalysisModule {
    pub default_client: Client,
    pub reasoning_client: Client,
    pub backend: Backend,
    pub xr: XapianReader,
    pub default_model: String,
    pub reasoning_model: String,
    pub ticket: String,
    pub module_info: ModuleInfo,
}

impl VedaQueueModule for BusinessProcessAnalysisModule {
    fn before_batch(&mut self, _size_batch: u32) -> Option<u32> {
        None
    }

    fn prepare(&mut self, queue_element: &mut Individual) -> Result<bool, PrepareError> {
        let op_id = queue_element.get_first_integer("op_id").unwrap_or_default();

        let res = prepare_queue_element(self, queue_element);

        if let Err(e) = self.module_info.put_info(op_id, op_id) {
            error!("failed to write module_info, op_id = {}, err = {:?}", op_id, e);
            return Err(PrepareError::Fatal);
        }

        res
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

fn prepare_queue_element(module: &mut BusinessProcessAnalysisModule, queue_element: &mut Individual) -> Result<bool, PrepareError> {
    let event_id = queue_element.get_first_literal("event_id").unwrap_or_default();
    let source = queue_element.get_first_literal("src").unwrap_or_default();

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

    let is_deleted = new_state.is_exists_bool("v-s:deleted", true);

    if is_deleted {
        return Ok(true);
    }

    let counter = new_state.get_first_integer("v-s:updateCounter").unwrap_or(-1);

    // Обработка в зависимости от типа индивида
    if new_state.any_exists("rdf:type", &[&"v-bpa:BusinessProcess".to_string()]) {
        if source == "BPA" {
            return Ok(true);
        }

        info!("Found a saved object of type 'v-bpa:BusinessProcess' with ID: {}", new_state.get_id());

        // Анализируем обоснованность бизнес-процесса
        if let Err(e) = analyze_process_justification(module, &mut new_state, &event_id) {
            error!("Error analyzing business process justification: {:?}", e);
        }
    } else if new_state.any_exists("rdf:type", &[&"v-bpa:ClusterizationAttempt".to_string()]) {
        if source == "BPA" {
            return Ok(true);
        }

        info!("Found a saved object of type 'v-bpa:ClusterizationAttempt' with ID: {}:{}", new_state.get_id(), counter);

        // Выполняем шаг кластеризации
        if let Err(e) = analyze_process_clusters(module, &mut new_state, &event_id) {
            error!("Error analyzing process clusters: {:?}", e);
        }
    } else if new_state.any_exists("rdf:type", &[&"v-bpa:ProcessCluster".to_string()]) {
        if counter > 1 {
            return Ok(true);
        }

        info!("Found new process cluster: {}", new_state.get_id());
        if let Err(e) = analyze_and_optimize_cluster(module, new_state.get_id(), &event_id) {
            error!("Error analyze and_optimize cluster: {:?}", e);
        }
    } else if new_state.any_exists("rdf:type", &[&"v-bpa:GenericProcessingRequest".to_string()]) {
        if source == "BPA" {
            return Ok(true);
        }

        info!("Found generic processing request: {}", new_state.get_id());
        if let Err(e) = process_generic_request(module, &mut new_state, &event_id) {
            error!("Error processing generic request: {:?}", e);
        }

        // Inside prepare_queue_element function, add new condition:
    } else if new_state.any_exists("rdf:type", &["v-bpa:PipelineRequest"]) {
        if source == "BPA" {
            return Ok(true);
        }

        // Проверяем тип запрашиваемого пайплайна
        if let Some(pipeline_type) = new_state.get_first_literal("v-bpa:pipeline") {
            match pipeline_type.as_str() {
                "v-bpa:rawDocumentExtractingAndStructuringPipeline" => {
                    if let Err(e) = raw_document_extracting_and_structuring(module, &mut new_state, &event_id) {
                        error!("Error processing raw document pipeline request: {:?}", e);
                    }
                },
                "v-bpa:businessProcessExtractionPipeline" => {
                    if let Err(e) = business_process_extraction_pipeline(module, &mut new_state, &event_id) {
                        error!("Error processing extraction pipeline request: {:?}", e);
                    }
                },
                _ => {
                    warn!("Unknown pipeline type: {}", pipeline_type);
                },
            }
        }
    } else if new_state.any_exists("rdf:type", &["v-bpa:ProcessDocument"]) {
        //if source == "BPA" {
        //    return Ok(true);
        //}

        if let Err(e) = handle_document_status(module, &mut new_state, &event_id) {
            error!("Error handling document status: {:?}", e);
        }
    }

    Ok(true)
}
