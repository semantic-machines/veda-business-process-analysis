use crate::response_schema::ResponseSchema;

#[test]
fn test_schema_validation() {
    // Тест схемы без обязательного поля type
    let invalid_json = r#"{
        "properties": {
            "test": { "mapping": "test:property" }
        }
    }"#;

    let result = ResponseSchema::from_json(invalid_json);
    assert!(result.is_err(), "Schema without type field should fail validation");

    // Тест схемы с неправильным типом
    let invalid_type_json = r#"{
        "type": "array",
        "properties": {
            "test": { "mapping": "test:property" }
        }
    }"#;

    let result = ResponseSchema::from_json(invalid_type_json);
    assert!(result.is_err(), "Schema with non-object type should fail validation");

    // Тест минимальной валидной схемы
    let minimal_json = r#"{
        "type": "object",
        "properties": {
            "test": { "mapping": "test:property" }
        }
    }"#;

    let result = ResponseSchema::from_json(minimal_json);
    assert!(result.is_ok(), "Valid minimal schema should pass validation");
}
