use crate::model::{Requirement, RequirementIndex};
use std::collections::HashSet;

pub fn extract_requirements(body: &str) -> Result<RequirementIndex, String> {
    let lines: Vec<_> = body.lines().enumerate().collect();
    let section = lines
        .iter()
        .position(|(_, line)| *line == "## Requirements")
        .ok_or("missing managed ## Requirements section")?;
    let mut requirements = Vec::new();
    let mut cursor = section + 1;
    while cursor < lines.len() {
        let (line_number, line) = lines[cursor];
        if line == "## Requirements" {
            return Err(format!(
                "duplicate ## Requirements section at line {}",
                line_number + 1
            ));
        }
        if line.starts_with("## ") {
            break;
        }
        if line.trim().is_empty() {
            cursor += 1;
            continue;
        }
        let id = line
            .strip_prefix("### `")
            .and_then(|value| value.strip_suffix('`'))
            .ok_or_else(|| format!("unexpected content at line {}", line_number + 1))?;
        if !is_valid_requirement_id(id) {
            return Err(format!(
                "invalid requirement ID at line {}: expected requirement.rNNN",
                line_number + 1
            ));
        }
        cursor += 1;
        let mut prose = Vec::new();
        while cursor < lines.len() && lines[cursor].1 != "#### Ontology elements" {
            if lines[cursor].1.starts_with("### ") || lines[cursor].1.starts_with("## ") {
                return Err(format!("requirement {id} is missing ontology references"));
            }
            if !lines[cursor].1.trim().is_empty() {
                prose.push(lines[cursor].1);
            }
            cursor += 1;
        }
        if prose.is_empty() {
            return Err(format!("requirement {id} has empty normative prose"));
        }
        if cursor == lines.len() {
            return Err(format!("requirement {id} is missing ontology references"));
        }
        cursor += 1;
        while cursor < lines.len() && lines[cursor].1.trim().is_empty() {
            cursor += 1;
        }
        if cursor >= lines.len() || !lines[cursor].1.starts_with('|') {
            return Err(format!(
                "requirement {id} has invalid ontology table header"
            ));
        }
        if table_cells(lines[cursor].1).as_deref() != Some(["Ontology element"].as_slice()) {
            return Err(format!(
                "requirement {id} has invalid ontology table header"
            ));
        }
        cursor += 1;
        while cursor < lines.len() && lines[cursor].1.trim().is_empty() {
            cursor += 1;
        }
        if cursor >= lines.len() || !strict_separator(lines[cursor].1) {
            return Err(format!(
                "requirement {id} has invalid Markdown table separator"
            ));
        }
        cursor += 1;
        let mut refs = Vec::new();
        while cursor < lines.len() && lines[cursor].1.starts_with('|') {
            let cells = table_cells(lines[cursor].1).ok_or_else(|| {
                format!(
                    "ontology reference row must contain exactly one cell at line {}",
                    lines[cursor].0 + 1
                )
            })?;
            if cells.len() != 1 {
                return Err(format!(
                    "ontology reference row must contain exactly one cell at line {}",
                    lines[cursor].0 + 1
                ));
            }
            let cell = cells[0];
            if cell.is_empty() {
                return Err(format!(
                    "missing ontology reference at line {}",
                    lines[cursor].0 + 1
                ));
            }
            let reference = cell
                .strip_prefix('`')
                .and_then(|value| value.strip_suffix('`'))
                .unwrap_or(cell);
            if !is_valid_ontology_id(reference) {
                return Err(format!(
                    "malformed ontology reference at line {}: {reference}",
                    lines[cursor].0 + 1
                ));
            }
            if refs.iter().any(|existing| existing == reference) {
                return Err(format!(
                    "duplicate ontology reference in requirement {id}: {reference}"
                ));
            }
            refs.push(reference.to_owned());
            cursor += 1;
        }
        if refs.is_empty() {
            return Err(format!("requirement {id} has no ontology references"));
        }
        if cursor < lines.len()
            && !lines[cursor].1.trim().is_empty()
            && !lines[cursor].1.starts_with("### ")
            && !lines[cursor].1.starts_with("## ")
        {
            return Err(format!(
                "unexpected content after ontology table at line {}",
                lines[cursor].0 + 1
            ));
        }
        requirements.push(Requirement {
            id: id.to_owned(),
            prose: prose.join("\n"),
            ontology_element_ids: refs,
        });
    }
    if requirements.is_empty() {
        return Err("requirements candidate contains no requirements".into());
    }
    let mut ids = HashSet::new();
    for requirement in &requirements {
        if !ids.insert(&requirement.id) {
            return Err(format!("duplicate requirement ID: {}", requirement.id));
        }
    }
    Ok(RequirementIndex { requirements })
}

fn strict_separator(line: &str) -> bool {
    table_cells(line).is_some_and(|cells| cells.len() == 1 && valid_separator_cell(cells[0]))
}

fn valid_separator_cell(cell: &str) -> bool {
    let cell = cell.strip_prefix(':').unwrap_or(cell);
    let cell = cell.strip_suffix(':').unwrap_or(cell);
    cell.len() >= 3 && cell.bytes().all(|byte| byte == b'-')
}

fn table_cells(line: &str) -> Option<Vec<&str>> {
    let inner = line.strip_prefix('|')?.strip_suffix('|')?;
    Some(inner.split('|').map(str::trim).collect())
}
fn is_valid_requirement_id(value: &str) -> bool {
    value
        .strip_prefix("requirement.r")
        .is_some_and(|suffix| suffix.len() == 3 && suffix.bytes().all(|byte| byte.is_ascii_digit()))
}
fn is_valid_ontology_id(value: &str) -> bool {
    ["concept.c", "property.p", "relationship.r", "constraint.k"]
        .iter()
        .any(|prefix| {
            value.strip_prefix(prefix).is_some_and(|suffix| {
                suffix.len() == 3 && suffix.bytes().all(|byte| byte.is_ascii_digit())
            })
        })
}
