// business_process_handler.rs

use openai_dive::v1::resources::chat::{ChatCompletionParametersBuilder, ChatCompletionResponseFormat, ChatMessage, ChatMessageContent, JsonSchemaBuilder};
use serde_json;
use tokio::runtime::Runtime;
use v_common::onto::individual::Individual;
use v_common::v_api::api_client::IndvOp;
use v_common::v_api::obj::ResultCode;

use crate::prompt_manager::get_system_prompt;
use crate::queue_processor::BusinessProcessAnalysisModule;
use crate::types::ProcessJustification;

/// Анализирует обоснованность бизнес-процесса на основе связанных документов
/// используя AI для оценки уровня обоснованности.
///
/// # Arguments
/// * `module` - Модуль анализа бизнес-процессов с настройками и клиентом AI
/// * `bp_individual` - Индивид бизнес-процесса для анализа
///
/// # Returns
/// * `Result<(), Box<dyn std::error::Error>>` - Результат анализа и сохранения оценки
pub fn analyze_process_justification(module: &mut BusinessProcessAnalysisModule, bp_individual: &mut Individual) -> Result<(), Box<dyn std::error::Error>> {
    // Получаем системный промпт из онтологии
    let mut system_prompt = get_system_prompt(module, "v-bpa:AnalyzeBusinessPrompt")?;

    // Добавляем инструкцию вернуть ответ в формате JSON
    system_prompt.push_str("\nПожалуйста, верни ответ в формате JSON, соответствующий указанной схеме.");

    // Извлекаем поля из объекта BusinessProcess
    let process_name = bp_individual.get_first_literal("v-bpa:processName").ok_or("Отсутствует название процесса")?;
    let process_description = bp_individual.get_first_literal("v-bpa:processDescription").unwrap_or_default();
    let process_participants = bp_individual.get_first_literal("v-bpa:processParticipant").unwrap_or_default();
    let responsible_department = bp_individual.get_first_literal("v-bpa:responsibleDepartment").unwrap_or_default();
    let process_frequency = bp_individual.get_first_literal("v-bpa:processFrequency").unwrap_or_default();
    let labor_costs = bp_individual.get_first_literal("v-bpa:laborCosts").unwrap_or_default();

    let documents = collect_related_documents(module, bp_individual)?;
    let documents_value: serde_json::Value = serde_json::to_value(documents.clone()).expect("Failed to convert documents to JSON Value");

    let user_content = serde_json::json!({
        "processName": process_name,
        "processDescription": process_description,
        "participants": process_participants,
        "responsibleDepartment": responsible_department,
        "frequency": process_frequency,
        "laborCosts": labor_costs,
        "justificationDocuments": documents_value
    });

    info!("Justification documents collected: {:?}", documents);
    info!("Process Name: {}", process_name);
    info!("Process Description: {}", process_description);
    info!("System Prompt: {}", system_prompt);
    info!("User Content: {}", user_content);
    info!("Using model: {}", module.model);

    let chat_parameters = prepare_chat_parameters(module.model.clone(), system_prompt, user_content)?;
    debug!("Parameters prepared for OpenAI: {:?}", chat_parameters);

    let rt = Runtime::new()?;
    rt.block_on(async {
        send_request_to_openai(module, chat_parameters, bp_individual).await?;
        Ok(())
    })
}

/// Собирает связанные документы обоснования для бизнес-процесса
///
/// # Arguments
/// * `module` - Модуль с доступом к хранилищу
/// * `bp_individual` - Индивид бизнес-процесса
///
/// # Returns
/// * `Result<Vec<serde_json::Value>, Box<dyn std::error::Error>>` - Список документов в JSON формате
fn collect_related_documents(module: &mut BusinessProcessAnalysisModule, bp_individual: &Individual) -> Result<Vec<serde_json::Value>, Box<dyn std::error::Error>> {
    let mut documents = Vec::new();

    let justification_refs = bp_individual.get_literals_nm("v-bpa:processJustification").unwrap_or_default();
    for ref_id in justification_refs {
        let mut document = Individual::default();
        if module.backend.storage.get_individual(&ref_id, &mut document) == ResultCode::Ok {
            document.parse_all();
            let document_json = serde_json::json!({
                "name": document.get_first_literal("v-bpa:documentName").unwrap_or_default(),
                "content": document.get_first_literal("v-bpa:documentContent").unwrap_or_default()
            });
            documents.push(document_json);
        } else {
            error!("Не удалось загрузить документ обоснования с ID: {}", ref_id);
        }
    }
    Ok(documents)
}

/// Подготавливает параметры для запроса к чат-модели AI
///
/// # Arguments
/// * `model` - Название модели AI
/// * `system_prompt` - Системный промпт для AI
/// * `user_content` - Контент для анализа
///
/// # Returns
/// * `Result<ChatCompletionParameters, Box<dyn std::error::Error>>` - Параметры для запроса
fn prepare_chat_parameters(
    model: String,
    system_prompt: String,
    user_content: serde_json::Value,
) -> Result<openai_dive::v1::resources::chat::ChatCompletionParameters, Box<dyn std::error::Error>> {
    let json_schema = serde_json::json!({
        "type": "object",
        "properties": {
            "level": {
                "type": "string",
                "description": "Уровень обоснованности процесса",
                "enum": [
                    "Полностью обоснован",
                    "Частично обоснован",
                    "Не обоснован"
                ]
            }
        },
        "required": ["level"],
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
                content: ChatMessageContent::Text(user_content.to_string()),
                name: None,
            },
        ])
        .response_format(ChatCompletionResponseFormat::JsonSchema(JsonSchemaBuilder::default().name("process_justification").schema(json_schema).strict(true).build()?))
        .build()?;

    Ok(parameters)
}

/// Отправляет запрос к API AI и обрабатывает ответ
///
/// # Arguments
/// * `module` - Модуль с клиентом AI и настройками
/// * `parameters` - Параметры запроса к AI
/// * `bp_individual` - Индивид бизнес-процесса для обновления
///
/// # Returns
/// * `Result<(), Box<dyn std::error::Error>>` - Результат обработки ответа
async fn send_request_to_openai(
    module: &mut BusinessProcessAnalysisModule,
    parameters: openai_dive::v1::resources::chat::ChatCompletionParameters,
    bp_individual: &mut Individual,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Sending request to OpenAI API");

    let result = module.client.chat().create(parameters).await?;
    debug!("Received response from OpenAI: {:?}", result);

    if let Some(choice) = result.choices.first() {
        if let ChatMessage::Assistant {
            content: Some(ChatMessageContent::Text(text)),
            ..
        } = &choice.message
        {
            info!("Received text response from OpenAI: {}", text);
            let process_justification: ProcessJustification = serde_json::from_str(text)?;
            info!("Parsed process justification from text: {:?}", process_justification);

            let justification_uri = process_justification.level.to_uri();
            bp_individual.set_uri("v-bpa:justificationLevel", justification_uri);

            if let Err(e) = module.backend.mstorage_api.update_or_err(&module.ticket, "BPA", "", IndvOp::Put, bp_individual) {
                return Err(std::io::Error::new(std::io::ErrorKind::Other, format!("Failed to update BusinessProcess object, err={:?}", e)).into());
            }
        } else {
            error!("Unexpected message format in response");
        }
    } else {
        error!("No choices in the response");
    }

    Ok(())
}
