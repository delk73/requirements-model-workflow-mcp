use crate::model::{ExecutionEvidenceIndex, ExecutionEvidenceResult};
use std::collections::HashSet;

pub fn extract_execution_evidence(body: &str) -> Result<ExecutionEvidenceIndex, String> {
    let lines: Vec<_> = body.lines().enumerate().collect();
    let section = lines
        .iter()
        .position(|(_, line)| *line == "## Execution Results")
        .ok_or("missing managed ## Execution Results section")?;
    let mut metadata = Vec::new();
    let mut cursor = section + 1;
    while cursor < lines.len() && !lines[cursor].1.starts_with("|") {
        let line = lines[cursor].1.trim();
        if !line.is_empty() {
            let (key, value) = line
                .split_once(':')
                .ok_or("invalid execution session metadata")?;
            metadata.push((key.trim(), value.trim()));
        }
        cursor += 1;
    }
    let value = |key: &str| {
        metadata
            .iter()
            .find(|(name, _)| *name == key)
            .map(|(_, value)| (*value).to_owned())
            .filter(|value| !value.is_empty())
            .ok_or_else(|| format!("missing required execution metadata: {key}"))
    };
    let verification_artifact = value("Verification artifact")?;
    let verification_revision = value("Verification revision")?;
    let repository_revision = value("Repository revision")?;
    let occurred_at = value("Occurred at")?;
    let context = value("Context")?;
    if !valid_sha256_revision(&verification_revision)
        || !valid_repository_revision(&repository_revision)
    {
        return Err("malformed repository revision".into());
    }
    let header = lines
        .get(cursor)
        .and_then(|(_, line)| cells(line))
        .ok_or("invalid execution results table header")?;
    if header != ["Verification", "Outcome"] {
        return Err("invalid execution results table header".into());
    }
    cursor += 1;
    if lines
        .get(cursor)
        .is_none_or(|(_, line)| !is_separator(line))
    {
        return Err("invalid execution results table separator".into());
    }
    cursor += 1;
    let mut results = Vec::new();
    let mut ids = HashSet::new();
    while cursor < lines.len() && lines[cursor].1.starts_with('|') {
        let row = cells(lines[cursor].1).ok_or("invalid execution result row")?;
        if row.len() != 2 || row.iter().any(|cell| cell.is_empty()) {
            return Err("invalid execution result row".into());
        }
        let verification_id = unquote(row[0]);
        if !valid_verification_id(&verification_id) {
            return Err("malformed verification ID".into());
        }
        let outcome = unquote(row[1]);
        if !matches!(outcome.as_str(), "passed" | "failed" | "error" | "skipped") {
            return Err("unsupported outcome".into());
        }
        if !ids.insert(verification_id.clone()) {
            return Err("duplicate verification result".into());
        }
        results.push(ExecutionEvidenceResult {
            verification_id,
            outcome,
        });
        cursor += 1;
    }
    if results.is_empty() {
        return Err("execution evidence contains no results".into());
    }
    Ok(ExecutionEvidenceIndex {
        verification_artifact,
        verification_revision,
        repository_revision,
        occurred_at,
        context,
        results,
    })
}

fn cells(line: &str) -> Option<Vec<&str>> {
    Some(
        line.strip_prefix('|')?
            .strip_suffix('|')?
            .split('|')
            .map(str::trim)
            .collect(),
    )
}
fn unquote(value: &str) -> String {
    value
        .strip_prefix('`')
        .and_then(|value| value.strip_suffix('`'))
        .unwrap_or(value)
        .to_owned()
}
fn is_separator(line: &str) -> bool {
    cells(line).is_some_and(|row| {
        row.len() == 2
            && row
                .iter()
                .all(|cell| cell.len() >= 3 && cell.bytes().all(|byte| byte == b'-'))
    })
}
fn valid_verification_id(value: &str) -> bool {
    value
        .strip_prefix("verification.v")
        .is_some_and(|suffix| suffix.len() == 3 && suffix.bytes().all(|byte| byte.is_ascii_digit()))
}
fn valid_sha256_revision(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|suffix| {
        suffix.len() == 64 && suffix.bytes().all(|byte| byte.is_ascii_hexdigit())
    })
}
fn valid_repository_revision(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}
