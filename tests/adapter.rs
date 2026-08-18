use serde_json::{json, Value};
use std::{
    fs,
    io::Write,
    process::{Command, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn stdio_adapter_exercises_all_slice_one_operations() {
    let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/raw-adc");
    let dir = std::env::temp_dir().join(format!(
        "rmwm-stdio-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&dir).unwrap();
    for name in ["requirements_model.yaml", "story.md"] {
        fs::copy(source.join(name), dir.join(name)).unwrap();
    }
    let mut manifest = fs::read_to_string(dir.join("requirements_model.yaml")).unwrap();
    let start = manifest.find("  raw-adc-domain-framing:").unwrap();
    let end = manifest[start..]
        .find("\n\n  raw-adc-domain-ontology:")
        .unwrap()
        + start;
    manifest.replace_range(start..end, "  raw-adc-domain-framing:\n    type: \"domain_framing\"\n    representation:\n      path: \"domain_framing.md\"\n      media_type: \"text/markdown\"\n      encoding: \"utf-8\"\n      line_endings: \"lf\"\n    accepted: null");
    fs::write(dir.join("requirements_model.yaml"), manifest).unwrap();
    let input = [
        json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"inspect_model_state","arguments":{}}}),
        json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"read_accepted_artifact","arguments":{"artifact_id":"raw-adc-story"}}}),
        json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"begin_candidate","arguments":{"model_id":"raw-adc","artifact_id":"raw-adc-domain-framing","artifact_type":"domain_framing","target_revision":null,"source_revisions":{"raw-adc-story":"sha256:d9fc45a0fae8dccf8c4a6ddc7f13d1c4604775b0d1f03abfa92d8f4ec1ffe0ae"}}}}),
        json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"stage_candidate","arguments":{"candidate":{"model_id":"raw-adc","artifact_id":"raw-adc-domain-framing","artifact_type":"domain_framing","target_revision":null,"source_revisions":{"raw-adc-story":"sha256:d9fc45a0fae8dccf8c4a6ddc7f13d1c4604775b0d1f03abfa92d8f4ec1ffe0ae"}},"body":"# Framing"}}}),
        json!({"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"read_accepted_artifact","arguments":{"artifact_id":"missing"}}}),
        json!({"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"begin_candidate","arguments":{"model_id":"raw-adc","artifact_id":"raw-adc-domain-framing","artifact_type":"domain_framing","target_revision":null,"source_revisions":{"raw-adc-story":"sha256:stale"}}}}),
        json!({"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"stage_candidate","arguments":{"candidate":{"model_id":"raw-adc","artifact_id":"raw-adc-domain-framing","artifact_type":"domain_framing","target_revision":null,"source_revisions":{"raw-adc-story":"sha256:d9fc45a0fae8dccf8c4a6ddc7f13d1c4604775b0d1f03abfa92d8f4ec1ffe0ae"}},"body":"# Framing again"}}}),
    ];
    let mut child = Command::new(env!("CARGO_BIN_EXE_requirements-model-workflow-mcp"))
        .arg(&dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    for request in input {
        writeln!(stdin, "{}", request).unwrap();
    }
    drop(stdin);
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    let responses: Vec<Value> = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(responses.len(), 7);
    for response in &responses[..4] {
        let result = response.get("result").unwrap();
        assert!(result.get("content").unwrap().is_array());
        assert!(result.get("structuredContent").is_some());
        assert_ne!(result.get("isError"), Some(&Value::Bool(true)));
    }
    for response in &responses[4..] {
        let error_result = response.get("result").unwrap();
        assert_eq!(error_result.get("isError"), Some(&Value::Bool(true)));
        assert!(response.get("error").is_none());
    }
    fs::remove_dir_all(dir).unwrap();
}
