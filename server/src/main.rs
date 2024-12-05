// main.rs

#[macro_use]
extern crate log;

use crate::queue_processor::BusinessProcessAnalysisModule;
use openai_dive::v1::api::Client;
use serde::{Deserialize, Serialize};
use v_common::ft_xapian::xapian_reader::XapianReader;
use v_common::init_module_log;
use v_common::module::info::ModuleInfo;
use v_common::module::module_impl::{init_log, Module};
use v_common::module::veda_backend::Backend;
use v_common::storage::common::StorageMode;

mod business_process_handler;
mod cluster_optimizer;
mod clustering_handler;
mod common;
mod prompt_manager;
mod queue_processor;
pub mod response_schema;
mod types;

mod clustering_common;
mod extractors;
mod generic_processing_handler;
mod pipeline;

#[derive(Debug, Serialize, Deserialize)]
struct ApiConfig {
    api_key: String,
    model: String,
    #[serde(default)]
    base_url: String,
}

fn main() -> std::io::Result<()> {
    init_module_log!("BUSINESS_PROCESS_ANALYSIS");

    // Читаем настройки из файла business-process-analysis.toml
    let settings = config::Config::builder().add_source(config::File::with_name("./config/business-process-analysis")).build().expect("Failed to read configuration");

    // Получаем название провайдера
    let provider = settings.get_string("provider").expect("Failed to get provider from config");

    // Читаем конфигурацию для выбранного провайдера
    let api_config: ApiConfig = settings.get(&provider).expect("Failed to get provider config");

    // Создаем клиент с учетом возможного base_url
    let client = Client {
        http_client: reqwest::Client::new(),
        base_url: if !api_config.base_url.is_empty() {
            api_config.base_url
        } else {
            "https://api.openai.com/v1".to_string()
        },
        api_key: api_config.api_key,
        headers: None,
        organization: None,
        project: None,
    };

    // Инициализируем бэкенд для доступа к хранилищу онтологии
    let mut backend = Backend::create(StorageMode::ReadOnly, false);

    // Инициализируем XapianReader
    let xr = XapianReader::new("russian", &mut backend.storage).expect("Failed to create XapianReader");

    let mut module = Module::new_with_name("business-process-analysis");

    let module_info = ModuleInfo::new("./data", "business-process-analysis", true);
    if module_info.is_err() {
        error!("failed to start, err = {:?}", module_info.err());
        return Ok(());
    }

    let systicket = if let Ok(t) = backend.get_sys_ticket_id() {
        t
    } else {
        error!("Cannot load sys ticket");
        return Ok(());
    };

    let mut my_module = BusinessProcessAnalysisModule {
        client,
        backend,
        xr,
        model: api_config.model,
        ticket: systicket,
        module_info: module_info.unwrap(),
    };

    module.prepare_queue(&mut my_module);

    Ok(())
}
