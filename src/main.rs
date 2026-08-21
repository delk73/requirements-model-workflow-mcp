use requirements_model_workflow_mcp::{
    model::{CandidateIdentity, StagedCandidate, StagedCandidateView},
    protocol::{JsonRpcRequest, JsonRpcResponse},
    store::ModelStore,
};
use serde_json::{json, Value};
use std::{
    env,
    io::{self, BufRead, Write},
    path::PathBuf,
};

const SERVER_INSTRUCTIONS: &str = "This process is an MCP STDIO server, not a CLI. Use the tools to inspect model state and accepted artifacts, validate a candidate with begin_candidate, stage it, begin review, and record an approved or rejected decision. stage_candidate, begin_candidate_review, and record_candidate_decision write workflow records under .rmwm; the other operations are read-only. Approval only records the review decision: it does not accept an artifact or modify the requirements-model manifest.";

fn candidate_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "model_id": {"type": "string"},
            "artifact_id": {"type": "string"},
            "artifact_type": {"type": "string"},
            "target_revision": {"type": ["string", "null"]},
            "source_revisions": {
                "type": "object",
                "patternProperties": {"^.+$": {"type": "string"}},
                "additionalProperties": false
            }
        },
        "required": [
            "model_id",
            "artifact_id",
            "artifact_type",
            "target_revision",
            "source_revisions"
        ],
        "additionalProperties": false
    })
}

fn tools() -> Value {
    let candidate = candidate_schema();
    json!({"tools": [
        {
            "name": "inspect_model_state",
            "description": "Inspect the requirements model and report each artifact's current lifecycle state without modifying it. descriptor.accepted.revision identifies the currently accepted artifact; candidate_revision identifies the staged or reviewed candidate.",
            "inputSchema": {
                "type": "object",
                "properties": {},
                "required": [],
                "additionalProperties": false
            }
        },
        {
            "name": "read_accepted_artifact",
            "description": "Read the exact UTF-8 text and accepted revision descriptor for one accepted artifact without modifying it.",
            "inputSchema": {
                "type": "object",
                "properties": {"artifact_id": {"type": "string"}},
                "required": ["artifact_id"],
                "additionalProperties": false
            }
        },
        {
            "name": "begin_candidate",
            "description": "Validate a proposed candidate identity and its target and source revision bindings without staging or modifying records.",
            "inputSchema": candidate.clone()
        },
        {
            "name": "stage_candidate",
            "description": "Construct and persist an exact candidate from its identity and body text, excluding RMWM front matter, as a new .rmwm staged-candidate record. The tool owns front-matter generation.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "candidate": candidate,
                    "body": {
                        "type": "string",
                        "description": "Candidate artifact body text, excluding RMWM front matter; the tool generates the front matter."
                    }
                },
                "required": ["candidate", "body"],
                "additionalProperties": false
            }
        },
        {
            "name": "read_staged_candidate",
            "description": "Read and validate the currently staged candidate's exact UTF-8 text for an artifact without modifying it.",
            "inputSchema": {
                "type": "object",
                "properties": {"artifact_id": {"type": "string"}},
                "required": ["artifact_id"],
                "additionalProperties": false
            }
        },
        {
            "name": "begin_candidate_review",
            "description": "Open review for one exact staged candidate revision and persist a .rmwm review-request record.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "artifact_id": {"type": "string"},
                    "candidate_revision": {"type": "string"}
                },
                "required": ["artifact_id", "candidate_revision"],
                "additionalProperties": false
            }
        },
        {
            "name": "record_candidate_decision",
            "description": "Persist an approved or rejected .rmwm review decision for one exact staged candidate; approval does not accept the artifact.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "artifact_id": {"type": "string"},
                    "candidate_revision": {"type": "string"},
                    "decision": {"type": "string", "enum": ["approved", "rejected"]},
                    "decided_by": {"type": "string"},
                    "rationale": {"type": "string"}
                },
                "required": [
                    "artifact_id",
                    "candidate_revision",
                    "decision",
                    "decided_by"
                ],
                "additionalProperties": false
            }
        }
    ]})
}

fn dispatch(store: &ModelStore, request: &JsonRpcRequest) -> Result<Value, String> {
    match request.method.as_str() {
        "initialize" => Ok(
            json!({"protocolVersion":"2025-11-25","capabilities":{"tools":{}},"serverInfo":{"name":"requirements-model-workflow-mcp","version":"0.1.0"},"instructions":SERVER_INSTRUCTIONS}),
        ),
        "ping" => Ok(json!({})),
        "tools/list" => Ok(tools()),
        "tools/call" => {
            let result = (|| -> Result<Value, String> {
                let name = request
                    .params
                    .get("name")
                    .and_then(Value::as_str)
                    .ok_or_else(|| "missing tool name".to_owned())?;
                let args = request
                    .params
                    .get("arguments")
                    .cloned()
                    .unwrap_or_else(|| json!({}));
                match name {
                    "inspect_model_state" => serde_json::to_value(store.inspect_model_state()?)
                        .map_err(|error| error.to_string()),
                    "read_accepted_artifact" => store.read_accepted_artifact(
                        args.get("artifact_id")
                            .and_then(Value::as_str)
                            .ok_or_else(|| "missing artifact_id".to_owned())?,
                    ),
                    "begin_candidate" => {
                        let identity: CandidateIdentity =
                            serde_json::from_value(args).map_err(|error| error.to_string())?;
                        serde_json::to_value(store.begin_candidate(identity)?)
                            .map_err(|error| error.to_string())
                    }
                    "stage_candidate" => {
                        let identity: CandidateIdentity = serde_json::from_value(
                            args.get("candidate")
                                .cloned()
                                .ok_or_else(|| "missing candidate".to_owned())?,
                        )
                        .map_err(|error| error.to_string())?;
                        let body = args
                            .get("body")
                            .and_then(Value::as_str)
                            .ok_or_else(|| "missing body".to_owned())?;
                        serde_json::to_value(staged_candidate_view(
                            store.stage_candidate(identity, body)?,
                        )?)
                        .map_err(|error| error.to_string())
                    }
                    "read_staged_candidate" => serde_json::to_value(staged_candidate_view(
                        store.read_staged_candidate(required_string(&args, "artifact_id")?)?,
                    )?)
                    .map_err(|error| error.to_string()),
                    "begin_candidate_review" => {
                        serde_json::to_value(store.begin_candidate_review(
                            required_string(&args, "artifact_id")?,
                            required_string(&args, "candidate_revision")?,
                        )?)
                        .map_err(|error| error.to_string())
                    }
                    "record_candidate_decision" => {
                        serde_json::to_value(store.record_candidate_decision(
                            required_string(&args, "artifact_id")?,
                            required_string(&args, "candidate_revision")?,
                            required_string(&args, "decision")?,
                            required_string(&args, "decided_by")?.into(),
                            optional_string(&args, "rationale")?,
                        )?)
                        .map_err(|error| error.to_string())
                    }
                    _ => Err(format!("unknown tool {name}")),
                }
            })();
            Ok(tool_result(result))
        }
        _ => Err(format!("method not found: {}", request.method)),
    }
}

fn staged_candidate_view(candidate: StagedCandidate) -> Result<StagedCandidateView, String> {
    let StagedCandidate {
        identity,
        bytes,
        content,
        revision,
        supersedes,
        state,
    } = candidate;
    let text = String::from_utf8(bytes).map_err(|error| error.to_string())?;
    Ok(StagedCandidateView {
        identity,
        text,
        content,
        revision,
        supersedes,
        state,
    })
}

fn required_string<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("missing {key}"))
}

fn optional_string(value: &Value, key: &str) -> Result<Option<String>, String> {
    match value.get(key) {
        None => Ok(None),
        Some(Value::String(value)) => Ok(Some(value.clone())),
        Some(_) => Err(format!("{key} must be a string")),
    }
}

fn tool_result(result: Result<Value, String>) -> Value {
    match result {
        Ok(value) => json!({
            "content": [{"type": "text", "text": serde_json::to_string(&value).unwrap()}],
            "structuredContent": value,
        }),
        Err(error) => json!({
            "content": [{"type": "text", "text": error}],
            "isError": true,
        }),
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .ok_or("usage: requirements-model-workflow-mcp MODEL_ROOT")?;
    let store = ModelStore::open(root);
    let stdin = io::stdin();
    let mut stdout = io::stdout().lock();
    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let request: JsonRpcRequest = match serde_json::from_str(&line) {
            Ok(request) => request,
            Err(error) => {
                writeln!(
                    stdout,
                    "{}",
                    serde_json::to_string(&JsonRpcResponse::failure(
                        Value::Null,
                        -32700,
                        error.to_string()
                    ))?
                )?;
                stdout.flush()?;
                continue;
            }
        };
        let Some(id) = request.id.clone() else {
            continue;
        };
        let response = if request.jsonrpc != "2.0" {
            JsonRpcResponse::failure(id, -32600, "only JSON-RPC 2.0 is supported")
        } else {
            match dispatch(&store, &request) {
                Ok(value) => JsonRpcResponse::success(id, value),
                Err(error) => JsonRpcResponse::failure(id, -32602, error),
            }
        };
        writeln!(stdout, "{}", serde_json::to_string(&response)?)?;
        stdout.flush()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::dispatch;
    use requirements_model_workflow_mcp::{model::CandidateIdentity, store::ModelStore};
    use serde_json::json;
    use std::{
        collections::BTreeMap,
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn model() -> (PathBuf, ModelStore, String) {
        let source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/raw-adc");
        let dir = std::env::temp_dir().join(format!(
            "rmwm-adapter-{}",
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
        let store = ModelStore::open(&dir);
        let story_revision = store
            .inspect_model_state()
            .unwrap()
            .artifacts
            .iter()
            .find(|artifact| artifact.artifact_id == "raw-adc-story")
            .unwrap()
            .descriptor
            .accepted
            .as_ref()
            .unwrap()
            .revision
            .clone();
        (dir, store, story_revision)
    }

    #[test]
    fn dispatch_exercises_all_slice_one_operations() {
        let (dir, store, story_revision) = model();
        let request = |name: &str, arguments: serde_json::Value| super::JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: "tools/call".into(),
            params: json!({"name": name, "arguments": arguments}),
        };
        assert!(dispatch(&store, &request("inspect_model_state", json!({}))).is_ok());
        assert!(dispatch(
            &store,
            &request(
                "read_accepted_artifact",
                json!({"artifact_id": "raw-adc-story"})
            )
        )
        .is_ok());
        let identity = CandidateIdentity {
            model_id: "raw-adc".into(),
            artifact_id: "raw-adc-domain-framing".into(),
            artifact_type: "domain_framing".into(),
            target_revision: None,
            source_revisions: BTreeMap::from([(String::from("raw-adc-story"), story_revision)]),
        };
        let identity_value = serde_json::to_value(&identity).unwrap();
        assert!(dispatch(&store, &request("begin_candidate", identity_value.clone())).is_ok());
        assert!(dispatch(
            &store,
            &request(
                "stage_candidate",
                json!({"candidate": identity_value, "body": "# Framing"})
            )
        )
        .is_ok());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn artifact_tool_responses_use_exact_text_without_byte_arrays() {
        let (dir, store, story_revision) = model();
        let request = |name: &str, arguments: serde_json::Value| super::JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: "tools/call".into(),
            params: json!({"name": name, "arguments": arguments}),
        };
        let accepted = dispatch(
            &store,
            &request(
                "read_accepted_artifact",
                json!({"artifact_id": "raw-adc-story"}),
            ),
        )
        .unwrap();
        assert_eq!(
            accepted["structuredContent"]["text"],
            json!(fs::read_to_string(dir.join("story.md")).unwrap())
        );
        assert!(accepted["structuredContent"].get("bytes").is_none());

        let identity = CandidateIdentity {
            model_id: "raw-adc".into(),
            artifact_id: "raw-adc-domain-framing".into(),
            artifact_type: "domain_framing".into(),
            target_revision: None,
            source_revisions: BTreeMap::from([(String::from("raw-adc-story"), story_revision)]),
        };
        let identity_value = serde_json::to_value(&identity).unwrap();
        let staged = dispatch(
            &store,
            &request(
                "stage_candidate",
                json!({"candidate": identity_value, "body": "# Exact\n\nBody"}),
            ),
        )
        .unwrap();
        assert_eq!(
            staged["structuredContent"]["text"],
            "---\nrmwm:\n  schema: \"artifact/v1\"\n  id: \"raw-adc-domain-framing\"\n  type: \"domain_framing\"\n---\n\n# Exact\n\nBody\n"
        );
        assert!(staged["structuredContent"].get("bytes").is_none());
        let read = dispatch(
            &store,
            &request(
                "read_staged_candidate",
                json!({"artifact_id": "raw-adc-domain-framing"}),
            ),
        )
        .unwrap();
        assert_eq!(
            read["structuredContent"]["text"],
            staged["structuredContent"]["text"]
        );
        assert!(read["structuredContent"].get("bytes").is_none());
        fs::remove_dir_all(dir).unwrap();
    }
}
