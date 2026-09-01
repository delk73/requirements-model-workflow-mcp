use crate::model::VocabularyOntologyReference;

pub fn extract_ontology_references(body: &str) -> Result<Vec<VocabularyOntologyReference>, String> {
    let mut references = Vec::new();
    let mut in_entries = false;
    let mut table_state = TableState::NotStarted;

    for (line_index, line) in body.lines().enumerate() {
        if line == "## Entries" {
            in_entries = true;
            table_state = TableState::NotStarted;
            continue;
        }
        if line.starts_with("## ") {
            in_entries = false;
            continue;
        }
        if !in_entries || !line.starts_with('|') {
            continue;
        }

        match table_state {
            TableState::NotStarted => table_state = TableState::HeaderSeen,
            TableState::HeaderSeen => {
                if !is_separator_row(line) {
                    return Err(format!(
                        "missing Markdown table separator in vocabulary entries at line {}",
                        line_index + 1
                    ));
                }
                table_state = TableState::Rows;
            }
            TableState::Rows => {
                let cell = first_table_cell(line).ok_or_else(|| {
                    format!(
                        "missing ontology reference at vocabulary entry line {}",
                        line_index + 1
                    )
                })?;
                let reference = cell
                    .strip_prefix('`')
                    .and_then(|value| value.strip_suffix('`'))
                    .unwrap_or(cell);
                if reference.is_empty() {
                    return Err(format!(
                        "missing ontology reference at vocabulary entry line {}",
                        line_index + 1
                    ));
                }
                if !is_valid_ontology_id(reference) {
                    return Err(format!(
                        "malformed ontology reference at vocabulary entry line {}: {reference}",
                        line_index + 1
                    ));
                }
                references.push(VocabularyOntologyReference {
                    ontology_element_id: reference.into(),
                });
            }
        }
    }

    if references.is_empty() {
        return Err("controlled vocabulary contains no entries with ontology references".into());
    }
    Ok(references)
}

pub fn admitted_ontology_ids(body: &str) -> Result<std::collections::HashSet<String>, String> {
    Ok(extract_ontology_references(body)?
        .into_iter()
        .map(|reference| reference.ontology_element_id)
        .collect())
}

#[derive(Clone, Copy)]
enum TableState {
    NotStarted,
    HeaderSeen,
    Rows,
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
    Some(cell)
}

fn is_valid_ontology_id(value: &str) -> bool {
    ["concept.c", "property.p", "relationship.r", "constraint.k"]
        .iter()
        .any(|prefix| {
            let Some(suffix) = value.strip_prefix(prefix) else {
                return false;
            };
            suffix.len() == 3 && suffix.bytes().all(|byte| byte.is_ascii_digit())
        })
}
