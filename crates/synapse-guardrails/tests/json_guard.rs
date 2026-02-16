use synapse_guardrails::{GuardrailError, JsonObjectGuard};

#[test]
fn accepts_json_object() {
    let value = JsonObjectGuard::validate("{\"ok\":true}").expect("should parse");
    assert_eq!(value["ok"], true);
}

#[test]
fn rejects_non_object_json() {
    let err = JsonObjectGuard::validate("[1,2,3]").expect_err("should reject arrays");
    assert!(matches!(err, GuardrailError::ExpectedObject));
}
