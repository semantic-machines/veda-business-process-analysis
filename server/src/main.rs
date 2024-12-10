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

mod ai_client;
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

mod document_status_handler;

mod process_structured_schema;

#[derive(Debug, Serialize, Deserialize)]
struct ApiConfig {
    api_key: String,
    model: String,
    #[serde(default)]
    base_url: String,
}

// Configuration for both providers
#[derive(Debug)]
pub struct ProvidersConfig {
    pub default: Client,
    pub reasoning: Client,
    pub default_model: String,
    pub reasoning_model: String,
}

fn create_client(api_config: &ApiConfig) -> Client {
    Client {
        http_client: reqwest::Client::new(),
        base_url: if !api_config.base_url.is_empty() {
            api_config.base_url.clone()
        } else {
            "https://api.openai.com/v1".to_string()
        },
        api_key: api_config.api_key.clone(),
        headers: None,
        organization: None,
        project: None,
    }
}

fn main() -> std::io::Result<()> {
    init_module_log!("BUSINESS_PROCESS_ANALYSIS");

    // Read settings from business-process-analysis.toml
    let settings = config::Config::builder().add_source(config::File::with_name("./config/business-process-analysis")).build().expect("Failed to read configuration");

    // Get both provider names
    let default_provider = settings.get_string("default_provider").expect("Failed to get default provider from config");
    let reasoning_provider = settings.get_string("reasoning_provider").expect("Failed to get reasoning provider from config");

    // Get configurations for both providers
    let default_config: ApiConfig = settings.get(&default_provider).expect("Failed to get default provider config");
    let reasoning_config: ApiConfig = settings.get(&reasoning_provider).expect("Failed to get reasoning provider config");

    // Create clients for both providers
    let default_client = create_client(&default_config);
    let reasoning_client = create_client(&reasoning_config);

    let providers_config = ProvidersConfig {
        default: default_client,
        reasoning: reasoning_client,
        default_model: default_config.model,
        reasoning_model: reasoning_config.model,
    };

    // Initialize backend for ontology storage access
    let mut backend = Backend::create(StorageMode::ReadOnly, false);

    // Initialize XapianReader
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
        default_client: providers_config.default,
        reasoning_client: providers_config.reasoning,
        backend,
        xr,
        default_model: providers_config.default_model,
        reasoning_model: providers_config.reasoning_model,
        ticket: systicket,
        module_info: module_info.unwrap(),
    };

    module.prepare_queue(&mut my_module);

    Ok(())
}
