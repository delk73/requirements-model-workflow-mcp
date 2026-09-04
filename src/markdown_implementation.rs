use crate::model::{ImplementationIndex, ImplementationTarget};
use std::{collections::HashSet, path::Path};

pub fn extract_implementations(body: &str) -> Result<ImplementationIndex, String> {
    let lines: Vec<_> = body.lines().enumerate().collect();
    let section = lines
        .iter()
        .position(|(_, line)| *line == "## Implementation Targets")
        .ok_or("missing managed ## Implementation Targets section")?;
    let mut cursor = section + 1;
    while cursor < lines.len() && lines[cursor].1.trim().is_empty() {
        cursor += 1;
    }
    let header = lines
        .get(cursor)
        .and_then(|(_, line)| cells(line))
        .ok_or("invalid implementation targets table header")?;
    if header != ["ID", "Repository revision", "Path", "Symbol"] {
        return Err("invalid implementation targets table header".into());
    }
    cursor += 1;
    if cursor >= lines.len() || !is_separator(lines[cursor].1) {
        return Err("invalid implementation targets table separator".into());
    }
    cursor += 1;
    let mut targets = Vec::new();
    let mut ids = HashSet::new();
    let mut seen_locators = HashSet::new();
    while cursor < lines.len() && lines[cursor].1.starts_with('|') {
        let row = cells(lines[cursor].1)
            .ok_or("implementation target row must contain exactly four cells")?;
        if row.len() != 4 || row.iter().any(|cell| cell.is_empty()) {
            return Err(format!(
                "invalid implementation target at line {}",
                lines[cursor].0 + 1
            ));
        }
        let target = ImplementationTarget {
            id: unquote(row[0]),
            repository_revision: unquote(row[1]),
            path: unquote(row[2]),
            symbol: unquote(row[3]),
        };
        if !valid_implementation_id(&target.id) {
            return Err(format!(
                "malformed implementation ID at line {}",
                lines[cursor].0 + 1
            ));
        }
        if !valid_commit_sha(&target.repository_revision) {
            return Err(format!(
                "malformed repository revision at line {}",
                lines[cursor].0 + 1
            ));
        }
        if !valid_repository_path(&target.path) {
            return Err(format!(
                "invalid implementation path at line {}",
                lines[cursor].0 + 1
            ));
        }
        if target.symbol.is_empty() {
            return Err(format!(
                "empty implementation symbol at line {}",
                lines[cursor].0 + 1
            ));
        }
        if !ids.insert(target.id.clone()) {
            return Err(format!("duplicate implementation ID: {}", target.id));
        }
        let locator = (
            target.repository_revision.clone(),
            target.path.clone(),
            target.symbol.clone(),
        );
        if !seen_locators.insert(locator) {
            return Err(format!(
                "duplicate implementation target at line {}",
                lines[cursor].0 + 1
            ));
        }
        targets.push(target);
        cursor += 1;
    }
    if targets.is_empty() {
        return Err("implementation candidate contains no targets".into());
    }
    Ok(ImplementationIndex { targets })
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

fn valid_implementation_id(value: &str) -> bool {
    value
        .strip_prefix("implementation.i")
        .is_some_and(|suffix| suffix.len() == 3 && suffix.bytes().all(|byte| byte.is_ascii_digit()))
}

fn valid_commit_sha(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn valid_repository_path(value: &str) -> bool {
    !value.is_empty()
        && !Path::new(value).is_absolute()
        && !value.starts_with('/')
        && !value.starts_with('\\')
        && !value.as_bytes().get(1).is_some_and(|byte| *byte == b':')
        && !value.split(['/', '\\']).any(|component| component == "..")
}

#[cfg(test)]
mod tests {
    use super::extract_implementations;

    const VALID: &str = "## Implementation Targets\n\n| ID | Repository revision | Path | Symbol |\n| --- | --- | --- | --- |\n| `implementation.i001` | 0123456789abcdef0123456789abcdef01234567 | `src/store.rs` | `ModelStore::accept_candidate` |\n";

    #[test]
    fn parses_valid_target() {
        assert_eq!(extract_implementations(VALID).unwrap().targets.len(), 1);
    }

    #[test]
    fn rejects_invalid_target_fields() {
        for replacement in [
            ("implementation.i001", "implementation.i01"),
            ("0123456789abcdef0123456789abcdef01234567", "not-a-sha"),
            ("`src/store.rs`", "`/src/store.rs`"),
            ("`src/store.rs`", "`../src/store.rs`"),
            ("`ModelStore::accept_candidate`", "``"),
        ] {
            assert!(extract_implementations(&VALID.replace(replacement.0, replacement.1)).is_err());
        }
    }

    #[test]
    fn rejects_duplicate_ids_and_targets() {
        let duplicate = format!("{VALID}| `implementation.i001` | 0123456789abcdef0123456789abcdef01234567 | `src/other.rs` | `other` |\n");
        assert!(extract_implementations(&duplicate).is_err());
        let duplicate = format!("{VALID}| `implementation.i002` | 0123456789abcdef0123456789abcdef01234567 | `src/store.rs` | `ModelStore::accept_candidate` |\n");
        assert!(extract_implementations(&duplicate).is_err());
    }
}
