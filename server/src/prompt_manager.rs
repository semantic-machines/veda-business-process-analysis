// prompt_manager.rs

use crate::queue_processor::BusinessProcessAnalysisModule;
use v_common::onto::individual::Individual;
use v_common::v_api::obj::ResultCode;

pub fn get_system_prompt(module: &mut BusinessProcessAnalysisModule, prompt_id: &str) -> Result<String, Box<dyn std::error::Error>> {
    // Получаем индивидуал промпта из хранилища
    let mut prompt_individual = Individual::default();
    if module.backend.storage.get_individual(prompt_id, &mut prompt_individual) != ResultCode::Ok {
        return Err(format!("Failed to get prompt with ID: {}", prompt_id).into());
    }

    prompt_individual.parse_all();

    // Получаем текст промпта
    let prompt_text = prompt_individual.get_first_literal("v-bpa:promptText").ok_or("Prompt text not found")?;

    Ok(prompt_text)
}
