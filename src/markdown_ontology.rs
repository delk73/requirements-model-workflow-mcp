use crate::model::{OntologyElement, OntologyElementIndex, OntologyElementKind};
use std::collections::HashSet;

pub fn extract_ontology_elements(body: &str) -> Result<OntologyElementIndex, String> {
    let mut elements = Vec::new();
    let mut section = None;
    let mut table_state = TableState::NotStarted;

    for (line_index, line) in body.lines().enumerate() {
        if let Some(kind) = heading_kind(line) {
            section = Some(kind);
            table_state = TableState::NotStarted;
            continue;
        }
        if line.starts_with("## ") {
            section = None;
            continue;
        }

        match section {
            Some(OntologyElementKind::Constraint) if line.starts_with("* ") => {
                let id = list_item_id(line).ok_or_else(|| {
                    format!(
                        "missing or malformed constraint ID at line {}",
                        line_index + 1
                    )
                })?;
                elements.push(OntologyElement {
                    id: validate_id(id, OntologyElementKind::Constraint, line_index + 1)?,
                    kind: OntologyElementKind::Constraint,
                });
            }
            Some(kind) if kind != OntologyElementKind::Constraint && line.starts_with('|') => {
                match table_state {
                    TableState::NotStarted => table_state = TableState::HeaderSeen,
                    TableState::HeaderSeen => {
                        if !is_separator_row(line) {
                            return Err(format!(
                                "missing Markdown table separator in {} at line {}",
                                kind.section_name(),
                                line_index + 1
                            ));
                        }
                        table_state = TableState::Rows;
                    }
                    TableState::Rows => {
                        let id = first_table_cell(line).ok_or_else(|| {
                            format!(
                                "missing {} ID at line {}",
                                kind.section_name().to_lowercase(),
                                line_index + 1
                            )
                        })?;
                        elements.push(OntologyElement {
                            id: validate_id(id, kind, line_index + 1)?,
                            kind,
                        });
                    }
                }
            }
            _ => {}
        }
    }

    if elements.is_empty() {
        return Err("ontology candidate contains no managed ontology elements".into());
    }
    validate_index(&elements)?;
    Ok(OntologyElementIndex { elements })
}

#[derive(Clone, Copy)]
enum TableState {
    NotStarted,
    HeaderSeen,
    Rows,
}

fn heading_kind(line: &str) -> Option<OntologyElementKind> {
    match line {
        "## Concepts" => Some(OntologyElementKind::Concept),
        "## Properties" => Some(OntologyElementKind::Property),
        "## Relationships" => Some(OntologyElementKind::Relationship),
        "## Constraints" => Some(OntologyElementKind::Constraint),
        _ => None,
    }
}

fn is_separator_row(line: &str) -> bool {
    line.split('|')
        .filter(|cell| !cell.trim().is_empty())
        .all(|cell| {
            let cell = cell.trim();
            cell.len() >= 3 && cell.bytes().all(|byte| matches!(byte, b'-' | b':' | b' '))
        })
}

fn first_table_cell(line: &str) -> Option<&str> {
    let cell = line.strip_prefix('|')?.split('|').next()?.trim();
    (!cell.is_empty()).then_some(cell)
}

fn list_item_id(line: &str) -> Option<&str> {
    line.strip_prefix("* ")?.split_ascii_whitespace().next()
}

fn validate_id(value: &str, kind: OntologyElementKind, line: usize) -> Result<String, String> {
    let value = value
        .strip_prefix('`')
        .and_then(|value| value.strip_suffix('`'))
        .unwrap_or(value);
    let suffix = value.strip_prefix(kind.id_prefix()).ok_or_else(|| {
        format!(
            "invalid {} ID at line {}: expected {}NNN",
            kind.section_name().to_lowercase(),
            line,
            kind.id_prefix()
        )
    })?;
    if suffix.len() != 3 || !suffix.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(format!(
            "invalid {} ID at line {}: expected {}NNN",
            kind.section_name().to_lowercase(),
            line,
            kind.id_prefix()
        ));
    }
    Ok(value.into())
}

fn validate_index(elements: &[OntologyElement]) -> Result<(), String> {
    let mut ids = HashSet::new();
    for element in elements {
        if !ids.insert(&element.id) {
            return Err(format!("duplicate ontology element ID: {}", element.id));
        }
    }
    Ok(())
}
