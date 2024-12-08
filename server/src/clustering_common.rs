use crate::common::{calculate_cost, save_to_interaction_file};
use crate::queue_processor::BusinessProcessAnalysisModule;
use openai_dive::v1::resources::chat::{ChatCompletionParametersBuilder, ChatCompletionResponseFormat, ChatMessage, ChatMessageContent, JsonSchemaBuilder};
use std::io;
use v_common::onto::individual::Individual;
use v_common::v_api::api_client::IndvOp;

/// Подготавливает параметры запроса для сравнения процессов
pub fn prepare_comparison_parameters(
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
        "required": ["are_similar"],
        "additionalProperties": false
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
pub async fn send_comparison_request(
    module: &mut BusinessProcessAnalysisModule,
    parameters: openai_dive::v1::resources::chat::ChatCompletionParameters,
) -> Result<bool, Box<dyn std::error::Error>> {

    save_to_interaction_file(&serde_json::to_string_pretty(&parameters)?, "comparison_request", "json")?;

    let result = module.default_client.chat().create(parameters).await?;

    if let Some(usage) = result.usage {
        info!(
            "API usage metrics - Tokens: input={}, output={}, total={}, cost={:.5}$",
            usage.prompt_tokens,
            usage.completion_tokens.unwrap_or(0),
            usage.total_tokens,
            calculate_cost(usage.total_tokens as f64, &module.default_model)
        );
    }

    if let Some(choice) = result.choices.first() {
        if let ChatMessage::Assistant {
            content: Some(ChatMessageContent::Text(text)),
            ..
        } = &choice.message
        {
            save_to_interaction_file(text, "comparison_response", "json")?;

            let response: serde_json::Value = serde_json::from_str(text)?;
            let similarity = response["are_similar"].as_bool().unwrap_or(false);
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

/// Вспомогательная функция для сохранения изменений в индивиде
pub fn update_individual(module: &mut BusinessProcessAnalysisModule, individual: &mut Individual, cmd: IndvOp) -> Result<(), Box<dyn std::error::Error>> {
    if let Err(e) = module.backend.mstorage_api.update_or_err(&module.ticket, "", "BPA", cmd, individual) {
        error!("Failed to update individual {}: {:?}", individual.get_id(), e);
        return Err(std::io::Error::new(io::ErrorKind::Other, format!("Failed to update individual, err={:?}", e)).into());
    }
    Ok(())
}
