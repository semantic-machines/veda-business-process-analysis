// main.rs

#[macro_use]
extern crate log;

use crate::queue_processor::BusinessProcessAnalysisModule;
use openai_dive::v1::api::Client;
use serde::{Deserialize, Serialize};
use v_common::ft_xapian::xapian_reader::XapianReader;
use v_common::init_module_log;
use v_common::module::module_impl::{init_log, Module};
use v_common::module::veda_backend::Backend;
use v_common::storage::common::StorageMode;

mod business_process_handler;
mod cluster_optimizer;
mod clustering_handler;
mod common;
mod prompt_manager;
mod queue_processor;
mod types;

mod generic_processing_handler;

#[derive(Debug, Serialize, Deserialize)]
struct Config {
    openai: OpenAIConfig,
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenAIConfig {
    api_key: String,
    model: String,
}

fn main() -> std::io::Result<()> {
    init_module_log!("BUSINESS_PROCESS_ANALYSIS");

    // Читаем настройки из файла business-process-analysis.toml
    let settings = config::Config::builder().add_source(config::File::with_name("business-process-analysis")).build().expect("Failed to read configuration");

    // Парсим настройки в структуру Config
    let config: Config = settings.try_deserialize().expect("Failed to deserialize configuration");

    // Инициализируем клиент OpenAI с использованием API ключа из настроек
    let client = Client::new(config.openai.api_key.clone());

    // Инициализируем бэкенд для доступа к хранилищу онтологии
    let mut backend = Backend::create(StorageMode::ReadOnly, false);

    // Инициализируем XapianReader
    let xr = XapianReader::new("russian", &mut backend.storage).expect("Failed to create XapianReader");

    let mut module = Module::new_with_name("business-process-analysis");

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
        model: config.openai.model.clone(),
        ticket: systicket,
    };

    module.prepare_queue(&mut my_module);

    Ok(())
}
