// main.rs

#[macro_use]
extern crate log;

use openai_dive::v1::api::Client;
use openai_dive::v1::resources::chat::{ChatCompletionParametersBuilder, ChatCompletionResponseFormat, ChatMessage, ChatMessageContent, JsonSchemaBuilder};
use serde::{Deserialize, Serialize};
use tokio::runtime::Runtime;
use v_common::init_module_log;
use v_common::module::module_impl::{get_inner_binobj_as_individual, Module, PrepareError};
use v_common::module::veda_backend::Backend;
use v_common::module::veda_module::VedaQueueModule;
use v_common::onto::individual::Individual;
use v_common::onto::parser::parse_raw;
use v_common::storage::common::StorageMode;
use v_common::v_api::obj::ResultCode;
use v_common::module::module_impl::init_log;
use v_common::v_api::api_client::IndvOp;

struct BusinessProcessAnalysisModule {
    client: Client,
    backend: Backend,
    model: String,
    ticket: String,
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
            if let Err(e) = self.process_business_process_async(&mut new_state) {
                error!("Error processing BusinessProcess: {:?}", e);
            }
        } else {
            //info!("Processing queue element with ID: {}", queue_element.get_id());
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

// Добавляем перечисление JustificationLevel с маппингом на текстовые метки из OpenAI
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
enum JustificationLevel {
    #[serde(rename = "Полностью обоснован")]
    CompletelyJustified,
    #[serde(rename = "Частично обоснован")]
    PartlyJustified,
    #[serde(rename = "Не обоснован")]
    NotJustified,
}

impl JustificationLevel {
    // Преобразование уровня обоснованности в URI онтологии
    fn to_uri(&self) -> &'static str {
        match self {
            JustificationLevel::CompletelyJustified => "v-bpa:CompletelyJustified",
            JustificationLevel::PartlyJustified => "v-bpa:PartlyJustified",
            JustificationLevel::NotJustified => "v-bpa:NotJustified",
        }
    }
}

// Структура для десериализации ответа OpenAI
#[derive(Debug, Serialize, Deserialize)]
struct ProcessJustification {
    level: JustificationLevel,
}

impl BusinessProcessAnalysisModule {
    fn collect_related_documents(&mut self, bp_individual: &Individual) -> Result<Vec<serde_json::Value>, Box<dyn std::error::Error>> {
        let mut documents = Vec::new();

        let justification_refs = bp_individual.get_literals_nm("v-bpa:processJustification").unwrap_or_default();
        for ref_id in justification_refs {
            let mut document = Individual::default();
            if self.backend.storage.get_individual(&ref_id, &mut document) == ResultCode::Ok {
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

    fn process_business_process_async(&mut self, bp_individual: &mut Individual) -> Result<(), Box<dyn std::error::Error>> {
        // Получаем системный промпт из онтологии
        let mut system_prompt = self.get_system_prompt("v-bpa:AnalyzeBusinessPrompt")?;

        // Добавляем инструкцию вернуть ответ в формате JSON
        system_prompt.push_str("\nПожалуйста, верни ответ в формате JSON, соответствующий указанной схеме.");

        // Извлекаем поля из объекта BusinessProcess
        let process_name = bp_individual.get_first_literal("v-bpa:processName").ok_or("Отсутствует название процесса")?;
        let process_description = bp_individual.get_first_literal("v-bpa:processDescription").unwrap_or_default();
        let process_participants = bp_individual.get_first_literal("v-bpa:processParticipant").unwrap_or_default();
        let responsible_department = bp_individual.get_first_literal("v-bpa:responsibleDepartment").unwrap_or_default();
        let process_frequency = bp_individual.get_first_literal("v-bpa:processFrequency").unwrap_or_default();
        let labor_costs = bp_individual.get_first_literal("v-bpa:laborCosts").unwrap_or_default();

        let documents = self.collect_related_documents(bp_individual)?;
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

        // Добавляем отладочную информацию
        info!("Process Name: {}", process_name);
        info!("Process Description: {}", process_description);
        info!("System Prompt: {}", system_prompt);
        info!("User Content: {}", user_content);
        info!("Using model: {}", self.model);

        // Определяем JSON-схему для структурированного вывода
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

        info!("Сформированный JSON для OpenAI: {}", user_content);

        // Создаем параметры для запроса, включая формат ответа
        let parameters = ChatCompletionParametersBuilder::default()
            .model(self.model.clone())
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
            .response_format(ChatCompletionResponseFormat::JsonSchema(
                JsonSchemaBuilder::default().name("process_justification").schema(json_schema).strict(true).build()?,
            ))
            .build()?;

        // Логируем параметры перед отправкой запроса
        debug!("Parameters sent to OpenAI: {:?}", parameters);

        // Отправляем запрос к OpenAI API асинхронно
        let rt = Runtime::new()?;
        rt.block_on(async {
            self.send_request_to_openai(parameters, bp_individual).await?;
            Ok(())
        })
    }

    async fn send_request_to_openai(
        &mut self,
        parameters: openai_dive::v1::resources::chat::ChatCompletionParameters,
        bp_individual: &mut Individual,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Логируем перед отправкой запроса
        info!("Sending request to OpenAI API");

        let result = self.client.chat().create(parameters).await?;

        // Логируем полученный ответ
        debug!("Received response from OpenAI: {:?}", result);

        // Обрабатываем ответ
        if let Some(choice) = result.choices.first() {
            if let ChatMessage::Assistant {
                content: Some(ChatMessageContent::Text(text)),
                ..
            } = &choice.message
            {
                // Логируем полученный текст
                info!("Received text response from OpenAI: {}", text);

                // Парсим текст как JSON
                let process_justification: ProcessJustification = serde_json::from_str(text)?;

                // Успешно распарсили
                info!("Parsed process justification from text: {:?}", process_justification);

                // Используем URI из онтологии вместо текстовой метки
                let justification_uri = process_justification.level.to_uri();

                // Устанавливаем новый уровень обоснованности, используя URI
                bp_individual.set_uri("v-bpa:justificationLevel", justification_uri);

                if let Err(e) = self.backend.mstorage_api.update_or_err(&self.ticket, "BPA", "", IndvOp::Put, bp_individual) {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("Failed to update BusinessProcess object, err={:?}", e)
                    ).into());
                }
            } else {
                error!("Unexpected message format in response");
            }
        } else {
            error!("No choices in the response");
        }

        Ok(())
    }

    fn get_system_prompt(&mut self, prompt_id: &str) -> Result<String, Box<dyn std::error::Error>> {
        // Получаем индивидуал промпта из хранилища
        let mut prompt_individual = Individual::default();
        if self.backend.storage.get_individual(prompt_id, &mut prompt_individual) != ResultCode::Ok {
            return Err(format!("Failed to get prompt with ID: {}", prompt_id).into());
        }

        prompt_individual.parse_all();

        // Получаем текст промпта
        let prompt_text = prompt_individual.get_first_literal("v-bpa:promptText").ok_or("Prompt text not found")?;

        Ok(prompt_text)
    }
}

// Структуры данных

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
    let settings = config::Config::builder()
        .add_source(config::File::with_name("business-process-analysis"))
        .build()
        .expect("Failed to read configuration");

    // Парсим настройки в структуру Config
    let config: Config = settings.try_deserialize().expect("Failed to deserialize configuration");

    // Инициализируем клиент OpenAI с использованием API ключа из настроек
    let client = Client::new(config.openai.api_key.clone());

    // Инициализируем бэкенд для доступа к хранилищу онтологии
    let mut backend = Backend::create(StorageMode::ReadOnly, false);

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
        model: config.openai.model.clone(),
        ticket: systicket,
    };

    module.prepare_queue(&mut my_module);

    Ok(())
}