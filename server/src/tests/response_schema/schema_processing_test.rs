use crate::response_schema::{ParseResult, ResponseSchema};
use rio_api::model::Literal::{LanguageTaggedString, Simple, Typed};
use rio_api::model::NamedOrBlankNode;
use rio_api::model::Term::{BlankNode, Literal, NamedNode};
use rio_api::parser::TriplesParser;
use rio_turtle::{TurtleError, TurtleParser};
use serde_json::{json, Value};
use std::fs;
use std::io::BufReader;
use std::path::Path;
use v_common::module::veda_backend::Backend;
use v_common::onto::datatype::Lang;
use v_common::onto::individual::Individual;
use v_common::storage::common::StorageMode;
use v_common::v_api::api_client::IndvOp;
use v_common::v_api::obj::ResultCode;

struct TestContext {
    backend: Backend,
    sys_ticket: String,
}

fn setup_test_context() -> TestContext {
    // Создаем бэкенд в режиме чтения-записи
    let mut backend = Backend::create(StorageMode::ReadWrite, false);

    // Получаем системный тикет
    let sys_ticket = backend
        .get_sys_ticket_id()
        .expect("Failed to get system ticket");

    // Создаем базовые классы и свойства
    let base_classes = [
        ("owl:Class", "rdf:type", "owl:Class"),
        ("owl:DatatypeProperty", "rdf:type", "owl:Class"),
        ("owl:FunctionalProperty", "rdf:type", "owl:Class"),
    ];

    for (id, pred, val) in base_classes.iter() {
        let mut indv = Individual::default();
        indv.set_id(id);
        indv.add_uri(pred, val);

        if backend
            .mstorage_api
            .update_or_err(&sys_ticket, "", "schema", IndvOp::Put, &mut indv)
            .is_err()
        {
            panic!("Failed to store base class {}", id);
        }
    }

    // Создаем определения свойств
    let properties = [
        ("v-bpa:documentSections", "owl:Class", true),
        ("v-bpa:sectionTitle", "xsd:string", false),
        ("v-bpa:sectionContent", "xsd:string", false),
        ("v-bpa:documentTitle", "xsd:string", false),
        ("v-bpa:documentType", "xsd:string", false),
        ("v-bpa:documentSource", "xsd:string", false),
        ("v-bpa:documentDate", "xsd:dateTime", false),
        ("v-bpa:documentSignedBy", "xsd:string", false),
        ("v-bpa:documentErrors", "xsd:string", true),
    ];

    for (uri, property_type, is_multiple) in properties.iter() {
        let mut individual = Individual::default();
        individual.set_id(uri);

        if *property_type == "owl:Class" {
            individual.add_uri("rdf:type", "owl:Class");
        } else {
            individual.add_uri("rdf:type", "owl:DatatypeProperty");
            individual.add_string("rdfs:range", property_type, Lang::none());

            if !is_multiple {
                individual.add_uri("rdf:type", "owl:FunctionalProperty");
            }
        }

        if backend
            .mstorage_api
            .update_or_err(&sys_ticket, "", "schema", IndvOp::Put, &mut individual)
            .is_err()
        {
            panic!("Failed to store property definition {}", uri);
        }

        // Проверяем сохранение
        let mut check_indv = Individual::default();
        if backend.storage.get_individual(uri, &mut check_indv) != ResultCode::Ok {
            panic!("Failed to read back property {}", uri);
        }
    }

    TestContext { backend, sys_ticket }
}

fn load_fixture(filename: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("tests")
        .join("response_schema")
        .join("fixtures")
        .join(filename);

    println!("Loading fixture from: {}", path.display());
    let content = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Failed to read fixture file {}: {}", filename, e));

    println!("Fixture content: {}", content);
    serde_json::from_str(&content)
        .unwrap_or_else(|e| panic!("Failed to parse JSON from fixture {}: {}", filename, e))
}

fn load_test_schema() -> ResponseSchema {
    println!("Loading test schema...");
    let schema_value = load_fixture("test_schema.json");
    println!("Schema value: {:#?}", schema_value);
    ResponseSchema::from_value(&schema_value).expect("Failed to parse test schema")
}

fn load_test_response() -> Value {
    println!("Loading test response...");
    let response = load_fixture("test_response.json");
    println!("Response value: {:#?}", response);
    response
}

fn load_expected_result() -> Individual {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("tests")
        .join("response_schema")
        .join("fixtures")
        .join("expected_result.ttl");

    println!("Loading expected result from: {}", path.display());
    let content = fs::read_to_string(&path).expect("Failed to read expected result file");
    println!("TTL content:\n{}", content);

    let file = BufReader::new(content.as_bytes());
    let mut parser = TurtleParser::new(file, None);
    let mut individual = Individual::default();
    let mut first = true;

    // Маппинг для сокращенных URI
    let namespaces = [
        ("rdf:", "http://www.w3.org/1999/02/22-rdf-syntax-ns#"),
        ("rdfs:", "http://www.w3.org/2000/01/rdf-schema#"),
        ("v-bpa:", "http://semantic-machines.com/veda/veda-business-process-analysis/"),
    ];

    println!("Starting RDF parsing...");

    fn shorten_uri(uri: &str, namespaces: &[(&str, &str)]) -> String {
        for (prefix, full) in namespaces {
            if uri.starts_with(full) {
                return format!("{}{}", prefix, &uri[full.len()..]);
            }
        }
        uri.to_string()
    }

    // Находим и обрабатываем тройки
    loop {
        let res: Result<(), TurtleError> = parser.parse_step(&mut |t| {
            println!("Processing triple: subject={:?}, predicate={:?}, object={:?}",
                     t.subject, t.predicate, t.object);

            let subject = match t.subject {
                NamedOrBlankNode::BlankNode(n) => n.id.to_string(),
                NamedOrBlankNode::NamedNode(n) => shorten_uri(n.iri, &namespaces),
            };

            // Устанавливаем id только для первого subject
            if first {
                println!("Setting individual ID to: {}", subject);
                individual.set_id(&subject);
                first = false;
            }

            let predicate = shorten_uri(t.predicate.iri, &namespaces);

            match t.object {
                BlankNode(_) => {
                    println!("Skipping blank node");
                    Ok(())
                },
                NamedNode(n) => {
                    let uri = shorten_uri(n.iri, &namespaces);
                    println!("Adding URI: {} -> {}", predicate, uri);
                    individual.add_uri(&predicate, &uri);
                    Ok(())
                },
                Literal(lit) => {
                    match lit {
                        Simple { value } => {
                            println!("Adding simple literal: {} -> {}", predicate, value);
                            individual.add_string(&predicate, value, Lang::none());
                        },
                        LanguageTaggedString { value, language } => {
                            println!("Adding language-tagged literal: {} -> {} @{}",
                                     predicate, value, language);
                            individual.add_string(&predicate, value, Lang::new_from_str(language));
                        },
                        Typed { value, datatype: _ } => {
                            println!("Adding typed literal: {} -> {}", predicate, value);
                            individual.add_string(&predicate, value, Lang::none());
                        },
                    }
                    Ok(())
                },
            }
        });

        if res.is_err() || parser.is_end() {
            println!("Parsing completed. Result: {:?}", res);
            break;
        }
    }

    println!("Final expected individual state:");
    println!("ID: {}", individual.get_id());
    println!("Properties: {:?}", individual.get_obj().get_resources());

    individual
}

fn compare_json_str(left: &str, right: &str) -> bool {
    let left_value: Result<Value, _> = serde_json::from_str(left);
    let right_value: Result<Value, _> = serde_json::from_str(right);

    if let (Ok(left_value), Ok(right_value)) = (left_value, right_value) {
        left_value == right_value
    } else {
        false
    }
}

fn verify_parsing_result(parse_result: &mut ParseResult, context: &mut TestContext) {
    println!("Loading expected result...");
    let expected = load_expected_result();
    println!("Expected individual id: {}", expected.get_id());
    println!("Expected properties: {:?}", expected.get_obj().get_resources());

    let main = &mut parse_result.main_individual;
    println!("Actual individual id: {}", main.get_id());
    println!("Actual properties: {:?}", main.get_obj().get_resources());

    // Проверяем свойства против ожидаемых значений
    for (predicate, resources) in expected.get_obj().get_resources().iter() {
        println!("\nChecking predicate: {}", predicate);
        println!("Expected resources: {:?}", resources);

        let actual_resources = main
            .get_obj()
            .get_resources()
            .get(predicate)
            .unwrap_or_else(|| panic!("Missing predicate: {}", predicate));

        println!("Actual resources: {:?}", actual_resources);

        assert_eq!(
            resources.len(),
            actual_resources.len(),
            "Different number of values for predicate: {}",
            predicate
        );

        for (idx, (expected_res, actual_res)) in resources.iter().zip(actual_resources.iter()).enumerate() {
            println!("Comparing value {} for predicate {}", idx, predicate);
            println!("Expected: {:?}", expected_res);
            println!("Actual: {:?}", actual_res);

            if predicate == "v-bpa:documentSections" {
                let equal = compare_json_str(expected_res.get_str(), actual_res.get_str());
                assert!(equal, "JSON values mismatch for predicate: {}\nExpected: {}\nActual: {}",
                        predicate, expected_res.get_str(), actual_res.get_str());
            } else {
                assert_eq!(
                    expected_res, actual_res,
                    "Value mismatch for predicate: {}",
                    predicate
                );
            }
        }
    }

    // Сохраняем основной индивид в хранилище
    if let Err(e) = context
        .backend
        .mstorage_api
        .update_or_err(&context.sys_ticket, "", "test", IndvOp::Put, main)
    {
        panic!("Failed to store main individual: {:?}", e);
    }
}

#[test]
fn test_schema_parsing_and_response_processing() {
    let mut context = setup_test_context();
    let test_schema = load_test_schema();

    // Проверяем создание схемы для AI
    let ai_schema = test_schema.to_ai_schema()
        .expect("Failed to create AI schema");
    println!("AI schema: {:#?}", ai_schema);

    let ai_schema_obj = ai_schema.as_object().unwrap();
    assert_eq!(
        ai_schema_obj.get("type").unwrap().as_str().unwrap(),
        "object",
        "Root schema type should be object"
    );
    assert!(
        ai_schema_obj.contains_key("properties"),
        "Schema should contain properties"
    );

    // Проверяем отсутствие служебных полей в схеме для AI
    let schema_str = serde_json::to_string(&ai_schema).unwrap();
    assert!(
        !schema_str.contains("mapping"),
        "AI schema should not contain mapping field"
    );
    assert!(
        !schema_str.contains("is_multiple"),
        "AI schema should not contain is_multiple field"
    );

    // Парсим ответ
    let mut parse_result = test_schema
        .parse_ai_response(&load_test_response(), &mut context.backend, &context.sys_ticket)
        .expect("Failed to parse AI response");

    verify_parsing_result(&mut parse_result, &mut context);
}

#[test]
fn test_invalid_schema() {
    println!("Testing invalid schema...");
    let invalid_schema = json!({
        "type": "array",
        "properties": {
            "test": { "mapping": "test:property" }
        }
    });

    let result = ResponseSchema::from_value(&invalid_schema);
    assert!(
        result.is_err(),
        "Schema with invalid root type should fail validation"
    );

    let incomplete_schema = json!({
        "type": "object"
    });

    let result = ResponseSchema::from_value(&incomplete_schema);
    assert!(
        result.is_err(),
        "Schema without required fields should fail validation"
    );
}

#[test]
fn test_schema_ai_conversion() {
    println!("Testing schema AI conversion...");
    let test_schema = load_test_schema();
    let ai_schema = test_schema.to_ai_schema().expect("Failed to create AI schema");
    println!("AI converted schema: {:#?}", ai_schema);

    fn check_no_service_fields(value: &Value) {
        if let Some(obj) = value.as_object() {
            assert!(
                !obj.contains_key("mapping"),
                "Schema should not contain mapping field"
            );
            assert!(
                !obj.contains_key("is_multiple"),
                "Schema should not contain is_multiple field"
            );

            for (_, v) in obj {
                check_no_service_fields(v);
            }
        } else if let Some(arr) = value.as_array() {
            for item in arr {
                check_no_service_fields(item);
            }
        }
    }

    check_no_service_fields(&ai_schema);
}