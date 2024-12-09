// ai_client.rs

use crate::common::ClientType;
use crate::queue_processor::BusinessProcessAnalysisModule;
use chrono::Utc;
use log::{error, info};
use openai_dive::v1::resources::chat::{ChatCompletionParameters, ChatMessage, ChatMessageContent};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::Path;

/// Base function for handling AI requests with common processing logic
async fn send_request_to_ai_base(
    module: &mut BusinessProcessAnalysisModule,
    parameters: ChatCompletionParameters,
    client_type: ClientType,
) -> Result<(String, usize, usize), Box<dyn std::error::Error>> {
    // Save request parameters to file for debugging
    save_to_interaction_file(&serde_json::to_string_pretty(&parameters)?, "request", "json")?;

    // Choose client based on type
    let result = match client_type {
        ClientType::Default => module.default_client.chat().create(parameters).await?,
        ClientType::Reasoning => module.reasoning_client.chat().create(parameters).await?,
    };

    // Extract token usage metrics
    let (input_tokens, output_tokens) = if let Some(usage) = result.usage {
        info!("API usage metrics - Tokens: input={}, output={}, total={}", usage.prompt_tokens, usage.completion_tokens.unwrap_or(0), usage.total_tokens);
        (usage.prompt_tokens as usize, usage.completion_tokens.unwrap_or(0) as usize)
    } else {
        (0, 0)
    };

    // Get response text from first choice
    if let Some(choice) = result.choices.first() {
        if let ChatMessage::Assistant {
            content: Some(ChatMessageContent::Text(text)),
            ..
        } = &choice.message
        {
            Ok((text.clone(), input_tokens, output_tokens))
        } else {
            error!("Unexpected message format in AI response");
            Err("Unexpected message format".into())
        }
    } else {
        error!("No response received from AI");
        Err("No response from AI".into())
    }
}

pub async fn send_text_request_to_ai(
    module: &mut BusinessProcessAnalysisModule,
    parameters: ChatCompletionParameters,
    client_type: ClientType,
) -> Result<AIResponseValues, Box<dyn std::error::Error>> {
    let (text, input_tokens, output_tokens) = send_request_to_ai_base(module, parameters, client_type).await?;

    // Save text response
    save_to_interaction_file(&text, "response", "txt")?;

    Ok(AIResponseValues::from_text(text, input_tokens, output_tokens))
}

pub async fn send_structured_request_to_ai(
    module: &mut BusinessProcessAnalysisModule,
    parameters: ChatCompletionParameters,
    client_type: ClientType,
) -> Result<AIResponseValues, Box<dyn std::error::Error>> {
    let (text, input_tokens, output_tokens) = send_request_to_ai_base(module, parameters, client_type).await?;

    // Save JSON response
    save_to_interaction_file(&text, "response", "json")?;

    // Parse response JSON
    let response: Value = serde_json::from_str(&text)?;
    let response_object = response.as_object().ok_or("Response is not a JSON object")?;

    let data: HashMap<String, Value> = response_object.iter().map(|(k, v)| (k.clone(), v.clone())).collect();

    Ok(AIResponseValues::new(data, input_tokens, output_tokens))
}

/// Saves data to file and returns path
pub fn save_to_interaction_file(data: &str, prefix: &str, extension: &str) -> Result<String, Box<dyn std::error::Error>> {
    // Create output directory if it doesn't exist
    let output_dir = "./ai_interactions";
    fs::create_dir_all(output_dir)?;

    // Generate unique filename with timestamp and prefix
    let filename = format!("{}_{}.{}", prefix, Utc::now().format("%Y%m%d_%H%M%S_%3f"), extension);
    let filepath = Path::new(output_dir).join(&filename);

    // Save data to file
    let mut file = File::create(&filepath)?;
    file.write_all(data.as_bytes())?;

    let request_path = filepath.to_string_lossy().into_owned();
    info!("AI request saved to: {}", request_path);

    Ok(request_path)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIResponseValues {
    pub data: HashMap<String, Value>,
    pub input_tokens: usize,
    pub output_tokens: usize,
}

impl AIResponseValues {
    pub fn new(data: HashMap<String, Value>, input_tokens: usize, output_tokens: usize) -> Self {
        Self {
            data,
            input_tokens,
            output_tokens,
        }
    }

    pub fn get(&self, key: &str) -> Option<&Value> {
        self.data.get(key)
    }

    //pub fn insert(&mut self, key: String, value: Value) {
    //    self.data.insert(key, value);
    //}

    // Helper method to create from single text result
    pub fn from_text(text: String, input_tokens: usize, output_tokens: usize) -> Self {
        let mut data = HashMap::new();
        data.insert("result".to_string(), Value::String(text));
        Self::new(data, input_tokens, output_tokens)
    }

    // Convert internal data to serde_json::Value
    pub fn to_json_value(&self) -> Value {
        let map: Map<String, Value> = self.data.clone().into_iter().collect();
        Value::Object(map)
    }
}
