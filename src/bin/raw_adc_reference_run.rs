//! Developer grounding harness: exercises the requirements-model-workflow-mcp
//! server through its real STDIO JSON-RPC surface against a temporary copy of
//! the Raw ADC example model. Run with `cargo run --bin raw_adc_reference_run`.
//!
//! This binary intentionally implements only the small JSON-RPC request/response
//! behavior needed for this one scenario; it is not a general MCP client.

use serde_json::{json, Value};
use std::{
    fs,
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};

const VOCABULARY: &str = "raw-adc-controlled-vocabulary";
const REQUIREMENTS: &str = "raw-adc-requirements";
const DECOMPOSITION: &str = "raw-adc-requirement-decomposition";

const REQUIREMENTS_PROSE_V1: &str =
    "The system shall identify each capture using a stable capture identity.";
const REQUIREMENTS_PROSE_V2: &str = "The system shall identify each capture using a stable \
capture identity that remains valid for the lifetime of the capture.";

fn main() {
    match run() {
        Ok(report) => {
            print_report(&report);
            println!("RESULT: PASS");
        }
        Err(error) => {
            eprintln!("Raw ADC MCP reference run failed: {error}");
            eprintln!("RESULT: FAIL");
            std::process::exit(1);
        }
    }
}

struct Report {
    model: TempModel,
    requirements_rev_old: String,
    requirements_rev_new: String,
    decomposition_rev: String,
    downstream_affected: Vec<String>,
}

fn print_report(report: &Report) {
    println!("Raw ADC MCP reference run");
    println!();
    println!("model: {}", report.model.path.display());
    println!("server: requirements-model-workflow-mcp");
    println!();
    println!("accepted:");
    println!("  requirements: {}", report.requirements_rev_new);
    println!("  requirement_decomposition: {}", report.decomposition_rev);
    println!();
    println!("lifecycle:");
    println!(
        "  requirements: {} -> {}",
        report.requirements_rev_old, report.requirements_rev_new
    );
    println!("  requirement_decomposition: accepted -> review_required");
    println!();
    println!("downstream impact:");
    for artifact_id in &report.downstream_affected {
        println!("  {artifact_id}");
    }
    println!();
}

fn run() -> Result<Report, String> {
    run_from_source(&default_raw_adc_source())
}

fn run_from_source(source_root: &Path) -> Result<Report, String> {
    let model = TempModel::create_from(source_root)?;
    let server_exe = resolve_server_binary()?;
    let mut client = Client::spawn(&server_exe, &model.path)?;
    match execute_scenario(&mut client, model) {
        Ok(report) => {
            client.finish()?;
            Ok(report)
        }
        Err(error) => {
            client.abort();
            Err(error)
        }
    }
}

fn default_raw_adc_source() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/raw-adc")
}

fn execute_scenario(client: &mut Client, model: TempModel) -> Result<Report, String> {
    let model_root = &model.path;
    client.expect_initialize()?;
    client.expect_tools_list()?;

    let state = client.call_tool("inspect_model_state", json!({}))?;
    let model_id = required_str(&state, "model_id")?.to_owned();
    let story_state = artifact_state(&state, "raw-adc-story")?;
    if story_state != "accepted" {
        return Err(format!(
            "raw-adc-story state is {story_state}; expected accepted"
        ));
    }
    // The checked-in Raw ADC example already carries an accepted controlled
    // vocabulary; only requirements and requirement decomposition are drafts.
    let vocabulary_rev = accepted_revision(&state, VOCABULARY)?;

    // B/C/D: accept the existing (unmodified) Raw ADC requirements content.
    let requirements_body_v1 = read_text(&model_root.join("requirements.md"))?;
    let requirements_rev_old = accept_candidate_flow(
        client,
        &model_id,
        REQUIREMENTS,
        "requirements",
        None,
        &[(VOCABULARY, &vocabulary_rev)],
        &requirements_body_v1,
    )?;

    // Accept the existing (unmodified) Raw ADC requirement decomposition.
    let decomposition_body = read_text(&model_root.join("requirement_decomposition.md"))?;
    let decomposition_rev = accept_candidate_flow(
        client,
        &model_id,
        DECOMPOSITION,
        "requirement_decomposition",
        None,
        &[(REQUIREMENTS, &requirements_rev_old)],
        &decomposition_body,
    )?;

    // E: confirm both artifacts are accepted before the upstream change.
    let mid_state = client.call_tool("inspect_model_state", json!({}))?;
    if artifact_state(&mid_state, REQUIREMENTS)? != "accepted" {
        return Err("requirements artifact is not accepted before upstream change".into());
    }
    if artifact_state(&mid_state, DECOMPOSITION)? != "accepted" {
        return Err(
            "requirement_decomposition artifact is not accepted before upstream change".into(),
        );
    }

    // F: reaccept the upstream requirements with a deliberate, valid content
    // change, preserving the requirement identity and ontology references.
    if !requirements_body_v1.contains(REQUIREMENTS_PROSE_V1) {
        return Err("requirements fixture is missing the expected prose to revise".into());
    }
    let requirements_body_v2 =
        requirements_body_v1.replacen(REQUIREMENTS_PROSE_V1, REQUIREMENTS_PROSE_V2, 1);
    let requirements_rev_new = accept_candidate_flow(
        client,
        &model_id,
        REQUIREMENTS,
        "requirements",
        Some(requirements_rev_old.clone()),
        &[(VOCABULARY, &vocabulary_rev)],
        &requirements_body_v2,
    )?;
    if requirements_rev_new == requirements_rev_old {
        return Err("upstream reacceptance did not produce a new revision".into());
    }

    // G: report direct downstream impact of the upstream requirements change.
    let impact = client.call_tool(
        "report_affected_downstream_artifacts",
        json!({"artifact_id": REQUIREMENTS}),
    )?;
    let affected_artifacts = impact
        .get("affected_artifacts")
        .and_then(Value::as_array)
        .ok_or("report_affected_downstream_artifacts missing affected_artifacts array")?;
    let downstream_affected: Vec<String> = affected_artifacts
        .iter()
        .filter_map(|entry| entry.get("artifact_id").and_then(Value::as_str))
        .map(String::from)
        .collect();
    if !downstream_affected.iter().any(|id| id == DECOMPOSITION) {
        return Err(
            "downstream impact report does not include the expected requirement_decomposition artifact"
                .into(),
        );
    }

    // H/I: confirm the accepted requirement decomposition became review_required.
    let final_state = client.call_tool("inspect_model_state", json!({}))?;
    let decomposition_final_state = artifact_state(&final_state, DECOMPOSITION)?;
    if decomposition_final_state != "review_required" {
        return Err(format!(
            "expected requirement_decomposition to become review_required, found {decomposition_final_state}"
        ));
    }

    Ok(Report {
        model,
        requirements_rev_old,
        requirements_rev_new,
        decomposition_rev,
        downstream_affected,
    })
}

/// Drives one artifact through begin_candidate -> stage_candidate ->
/// read_staged_candidate -> begin_candidate_review -> record_candidate_decision
/// -> accept_candidate, verifying the accepted revision matches the
/// staged/approved revision exactly. Returns the accepted revision.
#[allow(clippy::too_many_arguments)]
fn accept_candidate_flow(
    client: &mut Client,
    model_id: &str,
    artifact_id: &str,
    artifact_type: &str,
    target_revision: Option<String>,
    sources: &[(&str, &str)],
    body: &str,
) -> Result<String, String> {
    let source_revisions: serde_json::Map<String, Value> = sources
        .iter()
        .map(|(id, revision)| ((*id).to_owned(), json!(*revision)))
        .collect();
    let identity = json!({
        "model_id": model_id,
        "artifact_id": artifact_id,
        "artifact_type": artifact_type,
        "target_revision": target_revision,
        "source_revisions": Value::Object(source_revisions),
    });

    client.call_tool("begin_candidate", identity.clone())?;

    let staged = client.call_tool(
        "stage_candidate",
        json!({"candidate": identity, "body": body}),
    )?;
    let candidate_revision = required_str(&staged, "revision")?.to_owned();

    let read_back =
        client.call_tool("read_staged_candidate", json!({"artifact_id": artifact_id}))?;
    // The tool returns the full artifact text, including the RMWM front matter
    // the server generated, so compare against the submitted body as a suffix.
    if !required_str(&read_back, "text")?.ends_with(body) {
        return Err(format!(
            "staged candidate text for {artifact_id} does not match submitted body"
        ));
    }

    client.call_tool(
        "begin_candidate_review",
        json!({"artifact_id": artifact_id, "candidate_revision": candidate_revision}),
    )?;
    client.call_tool(
        "record_candidate_decision",
        json!({
            "artifact_id": artifact_id,
            "candidate_revision": candidate_revision,
            "decision": "approved",
            "decided_by": "raw_adc_reference_run",
        }),
    )?;
    let accepted = client.call_tool(
        "accept_candidate",
        json!({"artifact_id": artifact_id, "candidate_revision": candidate_revision}),
    )?;
    let accepted_revision = required_str(&accepted, "revision")?.to_owned();
    if accepted_revision != candidate_revision {
        return Err(format!(
            "accepted revision for {artifact_id} differs from the staged/approved revision"
        ));
    }
    if artifact_state(
        &client.call_tool("inspect_model_state", json!({}))?,
        artifact_id,
    )? != "accepted"
    {
        return Err(format!("{artifact_id} is not accepted after acceptance"));
    }
    Ok(accepted_revision)
}

fn find_artifact<'a>(state: &'a Value, artifact_id: &str) -> Result<&'a Value, String> {
    state
        .get("artifacts")
        .and_then(Value::as_array)
        .ok_or("model state missing artifacts array")?
        .iter()
        .find(|artifact| artifact.get("artifact_id").and_then(Value::as_str) == Some(artifact_id))
        .ok_or_else(|| format!("expected artifact ID is missing from model state: {artifact_id}"))
}

fn accepted_revision(state: &Value, artifact_id: &str) -> Result<String, String> {
    find_artifact(state, artifact_id)?
        .get("accepted")
        .and_then(|accepted| accepted.get("revision"))
        .and_then(Value::as_str)
        .map(String::from)
        .ok_or_else(|| format!("{artifact_id} has no accepted revision"))
}

fn artifact_state(state: &Value, artifact_id: &str) -> Result<String, String> {
    find_artifact(state, artifact_id)?
        .get("state")
        .and_then(Value::as_str)
        .map(String::from)
        .ok_or_else(|| format!("{artifact_id} is missing state"))
}

fn required_str<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("expected field missing or not a string: {key}"))
}

fn read_text(path: &Path) -> Result<String, String> {
    fs::read_to_string(path).map_err(|error| format!("failed to read {}: {error}", path.display()))
}

/// Minimal JSON-RPC 2.0 line-delimited STDIO client for this one scenario.
struct Client {
    child: Child,
    stdin: Option<ChildStdin>,
    reader: BufReader<ChildStdout>,
    next_id: i64,
}

impl Client {
    fn spawn(server_exe: &Path, model_root: &Path) -> Result<Self, String> {
        let mut child = Command::new(server_exe)
            .arg(model_root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|error| format!("failed to launch {}: {error}", server_exe.display()))?;
        let stdin = child
            .stdin
            .take()
            .ok_or("failed to open child server stdin")?;
        let stdout = child
            .stdout
            .take()
            .ok_or("failed to open child server stdout")?;
        Ok(Self {
            child,
            stdin: Some(stdin),
            reader: BufReader::new(stdout),
            next_id: 0,
        })
    }

    fn expect_initialize(&mut self) -> Result<(), String> {
        let result = self.call("initialize", json!({}))?;
        if result
            .get("protocolVersion")
            .and_then(Value::as_str)
            .is_none()
        {
            return Err("initialize response is missing protocolVersion".into());
        }
        Ok(())
    }

    fn expect_tools_list(&mut self) -> Result<(), String> {
        let result = self.call("tools/list", json!({}))?;
        let tool_names: Vec<String> = result
            .get("tools")
            .and_then(Value::as_array)
            .ok_or("tools/list response is missing tools array")?
            .iter()
            .filter_map(|tool| tool.get("name").and_then(Value::as_str))
            .map(String::from)
            .collect();
        let required_tools = [
            "inspect_model_state",
            "begin_candidate",
            "stage_candidate",
            "read_staged_candidate",
            "begin_candidate_review",
            "record_candidate_decision",
            "accept_candidate",
            "report_affected_downstream_artifacts",
        ];
        for tool in required_tools {
            if !tool_names.iter().any(|name| name == tool) {
                return Err(format!("expected tool missing from tools/list: {tool}"));
            }
        }
        Ok(())
    }

    fn call(&mut self, method: &str, params: Value) -> Result<Value, String> {
        let id = self.next_id;
        self.next_id += 1;
        let request = json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params});
        let line = serde_json::to_string(&request).map_err(|error| error.to_string())?;
        let stdin = self
            .stdin
            .as_mut()
            .ok_or("child server stdin is already closed")?;
        writeln!(stdin, "{line}")
            .map_err(|error| format!("failed writing {method} request: {error}"))?;
        stdin
            .flush()
            .map_err(|error| format!("failed flushing {method} request: {error}"))?;

        let mut response_line = String::new();
        let bytes_read = self
            .reader
            .read_line(&mut response_line)
            .map_err(|error| format!("failed reading response to {method}: {error}"))?;
        if bytes_read == 0 {
            return Err(format!(
                "child server exited unexpectedly while awaiting a response to {method}"
            ));
        }
        let response: Value = serde_json::from_str(response_line.trim_end())
            .map_err(|error| format!("malformed JSON response to {method}: {error}"))?;
        let response_id = response.get("id").cloned().unwrap_or(Value::Null);
        if response_id != json!(id) {
            return Err(format!(
                "response ID mismatch for {method}: expected {id}, got {response_id}"
            ));
        }
        if let Some(error) = response.get("error") {
            return Err(format!("JSON-RPC error response to {method}: {error}"));
        }
        Ok(response.get("result").cloned().unwrap_or(Value::Null))
    }

    fn call_tool(&mut self, name: &str, arguments: Value) -> Result<Value, String> {
        let result = self.call("tools/call", json!({"name": name, "arguments": arguments}))?;
        if result.get("isError") == Some(&Value::Bool(true)) {
            let text = result
                .get("content")
                .and_then(Value::as_array)
                .and_then(|content| content.first())
                .and_then(|entry| entry.get("text"))
                .and_then(Value::as_str)
                .unwrap_or("tool call failed");
            return Err(format!("tool {name} returned isError: {text}"));
        }
        result
            .get("structuredContent")
            .cloned()
            .ok_or_else(|| format!("tool {name} response is missing structuredContent"))
    }

    /// Closes stdin so the server observes EOF and exits cleanly, then waits.
    fn finish(mut self) -> Result<(), String> {
        self.stdin.take();
        let status = self
            .child
            .wait()
            .map_err(|error| format!("failed waiting for server to exit: {error}"))?;
        if !status.success() {
            return Err(format!("server exited with non-zero status: {status}"));
        }
        Ok(())
    }

    /// Forcibly terminates the child on a failed run.
    fn abort(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        // Best-effort safety net if neither finish() nor abort() ran (e.g. panic).
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// An isolated temporary copy of the Raw ADC example model, cleaned up on drop.
struct TempModel {
    path: PathBuf,
}

impl TempModel {
    fn create_from(source: &Path) -> Result<Self, String> {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| error.to_string())?
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "rmwm-raw-adc-reference-run-{}-{nanos}",
            std::process::id()
        ));
        fs::create_dir_all(&dir)
            .map_err(|error| format!("failed to create temporary model directory: {error}"))?;
        let model = Self { path: dir };
        for name in [
            "requirements_model.yaml",
            "story.md",
            "domain_framing.md",
            "domain_ontology.md",
            "controlled_vocabulary.md",
            "requirements.md",
            "requirement_decomposition.md",
        ] {
            fs::copy(source.join(name), model.path.join(name))
                .map_err(|error| format!("failed to copy {name} into temporary model: {error}"))?;
        }
        Ok(model)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_modified_accepted_story_in_temporary_fixture() {
        let source = TempModel::create_from(&default_raw_adc_source()).unwrap();
        let story_path = source.path.join("story.md");
        let mut story = fs::read_to_string(&story_path).unwrap();
        story.push_str("\nTemporary mutation for integrity testing.\n");
        fs::write(&story_path, story).unwrap();

        let error = match run_from_source(&source.path) {
            Ok(_) => panic!("tampered accepted story unexpectedly passed"),
            Err(error) => error,
        };

        assert!(
            error.contains("raw-adc-story state is modified; expected accepted"),
            "unexpected error: {error}"
        );
        assert!(!error.contains("Raw ADC MCP reference run"));
    }
}

impl Drop for TempModel {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

/// Resolves the workspace-built server binary deterministically from this
/// binary's own location, building it on demand if it is not yet present.
fn resolve_server_binary() -> Result<PathBuf, String> {
    let current_exe = std::env::current_exe()
        .map_err(|error| format!("failed to resolve current executable: {error}"))?;
    let dir = current_exe
        .parent()
        .ok_or("current executable has no parent directory")?;
    let dir = if dir.file_name().and_then(|name| name.to_str()) == Some("deps") {
        dir.parent().ok_or("Cargo deps directory has no parent")?
    } else {
        dir
    };
    let exe_name = if cfg!(windows) {
        "requirements-model-workflow-mcp.exe"
    } else {
        "requirements-model-workflow-mcp"
    };
    let candidate = dir.join(exe_name);

    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let mut build_args = vec![
        "build".to_owned(),
        "--bin".to_owned(),
        exe_name_without_ext(),
    ];
    if dir.file_name().and_then(|name| name.to_str()) == Some("release") {
        build_args.push("--release".to_owned());
    }
    let status = Command::new(&cargo)
        .args(&build_args)
        .status()
        .map_err(|error| format!("failed to build {exe_name}: {error}"))?;
    if !status.success() {
        return Err(format!(
            "cargo build for {exe_name} failed with status {status}"
        ));
    }
    if candidate.is_file() {
        Ok(candidate)
    } else {
        Err(format!(
            "server binary not found after build: {}",
            candidate.display()
        ))
    }
}

fn exe_name_without_ext() -> String {
    "requirements-model-workflow-mcp".to_owned()
}
