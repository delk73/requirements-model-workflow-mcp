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
        json!({"jsonrpc":"2.0","id":0,"method":"initialize","params":{}}),
        json!({"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}),
        json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"inspect_model_state","arguments":{}}}),
        json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"read_accepted_artifact","arguments":{"artifact_id":"raw-adc-story"}}}),
        json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"begin_candidate","arguments":{"model_id":"raw-adc","artifact_id":"raw-adc-domain-framing","artifact_type":"domain_framing","target_revision":null,"source_revisions":{"raw-adc-story":"sha256:d9fc45a0fae8dccf8c4a6ddc7f13d1c4604775b0d1f03abfa92d8f4ec1ffe0ae"}}}}),
        json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"stage_candidate","arguments":{"candidate":{"model_id":"raw-adc","artifact_id":"raw-adc-domain-framing","artifact_type":"domain_framing","target_revision":null,"source_revisions":{"raw-adc-story":"sha256:d9fc45a0fae8dccf8c4a6ddc7f13d1c4604775b0d1f03abfa92d8f4ec1ffe0ae"}},"body":"# Framing"}}}),
        json!({"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"read_staged_candidate","arguments":{"artifact_id":"raw-adc-domain-framing"}}}),
        json!({"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"begin_candidate_review","arguments":{"artifact_id":"raw-adc-domain-framing","candidate_revision":"sha256:PLACEHOLDER"}}}),
        json!({"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"record_candidate_decision","arguments":{"artifact_id":"raw-adc-domain-framing","candidate_revision":"sha256:PLACEHOLDER","decision":"approved","decided_by":"reviewer"}}}),
        json!({"jsonrpc":"2.0","id":8,"method":"tools/call","params":{"name":"read_accepted_artifact","arguments":{"artifact_id":"missing"}}}),
        json!({"jsonrpc":"2.0","id":9,"method":"tools/call","params":{"name":"begin_candidate","arguments":{"model_id":"raw-adc","artifact_id":"raw-adc-domain-framing","artifact_type":"domain_framing","target_revision":null,"source_revisions":{"raw-adc-story":"sha256:stale"}}}}),
        json!({"jsonrpc":"2.0","id":10,"method":"tools/call","params":{"name":"stage_candidate","arguments":{"candidate":{"model_id":"raw-adc","artifact_id":"raw-adc-domain-framing","artifact_type":"domain_framing","target_revision":null,"source_revisions":{"raw-adc-story":"sha256:d9fc45a0fae8dccf8c4a6ddc7f13d1c4604775b0d1f03abfa92d8f4ec1ffe0ae"}},"body":"# Framing again"}}}),
        json!({"jsonrpc":"2.0","id":11,"method":"tools/call","params":{"name":"read_staged_candidate","arguments":{"artifact_id":"missing"}}}),
        json!({"jsonrpc":"2.0","id":12,"method":"tools/call","params":{"name":"begin_candidate_review","arguments":{"artifact_id":"raw-adc-domain-framing","candidate_revision":"sha256:wrong"}}}),
        json!({"jsonrpc":"2.0","id":13,"method":"tools/call","params":{"name":"record_candidate_decision","arguments":{"artifact_id":"raw-adc-domain-framing","candidate_revision":"sha256:PLACEHOLDER","decision":"rejected","decided_by":"reviewer"}}}),
    ];
    let mut child = Command::new(env!("CARGO_BIN_EXE_requirements-model-workflow-mcp"))
        .arg(&dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let staged_revision = requirements_model_workflow_mcp::digest::revision_handle(
        "raw-adc",
        "raw-adc-domain-framing",
        "domain_framing",
        "domain_framing",
        &requirements_model_workflow_mcp::digest::content_digest(b"---\nrmwm:\n  schema: \"artifact/v1\"\n  id: \"raw-adc-domain-framing\"\n  type: \"domain_framing\"\n---\n# Framing\n"),
        &[("raw-adc-story".into(), "sha256:d9fc45a0fae8dccf8c4a6ddc7f13d1c4604775b0d1f03abfa92d8f4ec1ffe0ae".into())],
    );
    for request in input {
        let request = request
            .to_string()
            .replace("sha256:PLACEHOLDER", &staged_revision);
        writeln!(stdin, "{request}").unwrap();
    }
    drop(stdin);
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    let responses: Vec<Value> = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(responses.len(), 15);

    let instructions = responses[0]["result"]["instructions"].as_str().unwrap();
    assert!(instructions.contains("MCP STDIO server, not a CLI"));
    assert!(instructions.contains("write workflow records under .rmwm"));
    assert!(instructions.contains("does not accept an artifact or modify"));

    let tools = responses[1]["result"]["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 7);
    for tool in tools {
        assert!(!tool["description"].as_str().unwrap().is_empty());
        let schema = &tool["inputSchema"];
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"].is_object());
        assert!(schema["required"].is_array());
        assert_eq!(schema["additionalProperties"], false);
    }
    let stage = tools
        .iter()
        .find(|tool| tool["name"] == "stage_candidate")
        .unwrap();
    let candidate = &stage["inputSchema"]["properties"]["candidate"];
    assert!(candidate["properties"].is_object());
    assert_eq!(candidate["additionalProperties"], false);
    assert_eq!(
        candidate["properties"]["source_revisions"]["additionalProperties"],
        false
    );
    let decision = tools
        .iter()
        .find(|tool| tool["name"] == "record_candidate_decision")
        .unwrap();
    assert_eq!(
        decision["inputSchema"]["properties"]["decision"]["enum"],
        json!(["approved", "rejected"])
    );

    for response in &responses[2..9] {
        let result = response.get("result").unwrap();
        assert!(result.get("content").unwrap().is_array());
        assert!(result.get("structuredContent").is_some());
        assert_ne!(result.get("isError"), Some(&Value::Bool(true)));
    }
    for response in &responses[9..] {
        let error_result = response.get("result").unwrap();
        assert_eq!(error_result.get("isError"), Some(&Value::Bool(true)));
        assert!(response.get("error").is_none());
    }
    fs::remove_dir_all(dir).unwrap();
}
