use crate::model::{VerificationIndex, VerificationTarget};
use std::{collections::HashSet, path::Path};

pub fn extract_verifications(body: &str) -> Result<VerificationIndex, String> {
    let lines: Vec<_> = body.lines().enumerate().collect();
    let section = lines
        .iter()
        .position(|(_, line)| *line == "## Verification Targets")
        .ok_or("missing managed ## Verification Targets section")?;
    let mut cursor = section + 1;
    while cursor < lines.len() && lines[cursor].1.trim().is_empty() {
        cursor += 1;
    }
    let header = lines
        .get(cursor)
        .and_then(|(_, line)| cells(line))
        .ok_or("invalid verification targets table header")?;
    if header != ["ID", "Repository revision", "Path", "Test"] {
        return Err("invalid verification targets table header".into());
    }
    cursor += 1;
    if cursor >= lines.len() || !is_separator(lines[cursor].1) {
        return Err("invalid verification targets table separator".into());
    }
    cursor += 1;
    let mut targets = Vec::new();
    let mut ids = HashSet::new();
    let mut seen_targets = HashSet::new();
    while cursor < lines.len() && lines[cursor].1.starts_with('|') {
        let row = cells(lines[cursor].1)
            .ok_or("verification target row must contain exactly four cells")?;
        if row.len() != 4 || row.iter().any(|cell| cell.is_empty()) {
            return Err(format!(
                "invalid verification target at line {}",
                lines[cursor].0 + 1
            ));
        }
        let target = VerificationTarget {
            id: unquote(row[0]),
            repository_revision: unquote(row[1]),
            path: unquote(row[2]),
            test: unquote(row[3]),
        };
        if !valid_id(&target.id) {
            return Err(format!(
                "malformed verification ID at line {}",
                lines[cursor].0 + 1
            ));
        }
        if target.repository_revision.len() != 40
            || !target
                .repository_revision
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(format!(
                "malformed repository revision at line {}",
                lines[cursor].0 + 1
            ));
        }
        if !valid_path(&target.path) {
            return Err(format!(
                "invalid verification path at line {}",
                lines[cursor].0 + 1
            ));
        }
        if target.test.is_empty() {
            return Err(format!(
                "empty verification test at line {}",
                lines[cursor].0 + 1
            ));
        }
        if !ids.insert(target.id.clone()) {
            return Err(format!("duplicate verification ID: {}", target.id));
        }
        if !seen_targets.insert((
            target.repository_revision.clone(),
            target.path.clone(),
            target.test.clone(),
        )) {
            return Err(format!(
                "duplicate verification target at line {}",
                lines[cursor].0 + 1
            ));
        }
        targets.push(target);
        cursor += 1;
    }
    if targets.is_empty() {
        return Err("verification candidate contains no targets".into());
    }
    Ok(VerificationIndex { targets })
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
        row.len() == 4
            && row
                .iter()
                .all(|cell| cell.len() >= 3 && cell.bytes().all(|byte| byte == b'-'))
    })
}
fn valid_id(value: &str) -> bool {
    value
        .strip_prefix("verification.v")
        .is_some_and(|suffix| suffix.len() == 3 && suffix.bytes().all(|byte| byte.is_ascii_digit()))
}
fn valid_path(value: &str) -> bool {
    !value.is_empty()
        && !Path::new(value).is_absolute()
        && !value.starts_with('/')
        && !value.starts_with('\\')
        && !value.as_bytes().get(1).is_some_and(|byte| *byte == b':')
        && !value.split(['/', '\\']).any(|component| component == "..")
}
