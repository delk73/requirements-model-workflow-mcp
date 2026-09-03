use crate::model::{
    RequirementDecomposition, RequirementDecompositionIndex, RequirementDecompositionOutcome,
};
use std::collections::HashSet;

pub fn extract_requirement_decomposition(
    body: &str,
) -> Result<RequirementDecompositionIndex, String> {
    let lines: Vec<_> = body.lines().enumerate().collect();
    let sections: Vec<_> = lines
        .iter()
        .enumerate()
        .filter(|(_, (_, line))| *line == "## Decomposition")
        .map(|(index, _)| index)
        .collect();
    if sections.is_empty() {
        return Err("missing managed ## Decomposition section".into());
    }
    if sections.len() > 1 {
        return Err("duplicate ## Decomposition section".into());
    }
    let mut cursor = sections[0] + 1;
    let mut parents = Vec::new();
    while cursor < lines.len() && !lines[cursor].1.starts_with("## ") {
        if lines[cursor].1.trim().is_empty() {
            cursor += 1;
            continue;
        }
        let line_number = lines[cursor].0 + 1;
        let parent_id = lines[cursor]
            .1
            .strip_prefix("### `")
            .and_then(|value| value.strip_suffix('`'))
            .ok_or_else(|| format!("unexpected content at line {line_number}"))?;
        if !valid_requirement_id(parent_id) {
            return Err(format!(
                "malformed parent requirement ID at line {line_number}"
            ));
        }
        cursor += 1;
        let mut child_ids = Vec::new();
        let mut ontology_basis_ids = Vec::new();
        let mut rationale = Vec::new();
        let mut has_children = false;
        let mut has_atomic = false;
        let mut rationale_seen = false;
        let mut subsection_order = 0;
        while cursor < lines.len()
            && !lines[cursor].1.starts_with("### ")
            && !lines[cursor].1.starts_with("## ")
        {
            let line = lines[cursor].1;
            if line == "#### Child requirements" {
                if has_children || has_atomic || subsection_order != 0 {
                    return Err(format!("parent {parent_id} has multiple outcomes"));
                }
                has_children = true;
                subsection_order = 1;
                cursor += 1;
                while cursor < lines.len() && lines[cursor].1.trim().is_empty() {
                    cursor += 1;
                }
                while cursor < lines.len() && lines[cursor].1.starts_with("- ") {
                    let child = lines[cursor].1[2..].trim();
                    let child = child
                        .strip_prefix('`')
                        .and_then(|v| v.strip_suffix('`'))
                        .unwrap_or(child);
                    if !valid_requirement_id(child) {
                        return Err(format!(
                            "malformed child requirement ID at line {}",
                            lines[cursor].0 + 1
                        ));
                    }
                    if child_ids.iter().any(|existing| existing == child) {
                        return Err(format!(
                            "duplicate child reference in parent {parent_id}: {child}"
                        ));
                    }
                    child_ids.push(child.to_owned());
                    cursor += 1;
                }
                if child_ids.is_empty() {
                    return Err(format!("parent {parent_id} has an empty child list"));
                }
            } else if line == "#### No further decomposition" {
                if has_children || has_atomic {
                    return Err(format!("parent {parent_id} has multiple outcomes"));
                }
                has_atomic = true;
                cursor += 1;
            } else if line == "#### Ontology basis" {
                if !has_children || subsection_order >= 2 {
                    return Err(format!("invalid subsection order for parent {parent_id}"));
                }
                cursor += 1;
                subsection_order = 2;
                while cursor < lines.len() && lines[cursor].1.trim().is_empty() {
                    cursor += 1;
                }
                while cursor < lines.len() && lines[cursor].1.starts_with("- ") {
                    let reference = lines[cursor].1[2..].trim();
                    let reference = reference
                        .strip_prefix('`')
                        .and_then(|v| v.strip_suffix('`'))
                        .unwrap_or(reference);
                    if !valid_ontology_id(reference) {
                        return Err(format!(
                            "malformed ontology basis ID at line {}",
                            lines[cursor].0 + 1
                        ));
                    }
                    if ontology_basis_ids
                        .iter()
                        .any(|existing| existing == reference)
                    {
                        return Err(format!("duplicate ontology basis reference: {reference}"));
                    }
                    ontology_basis_ids.push(reference.to_owned());
                    cursor += 1;
                }
                if ontology_basis_ids.is_empty() {
                    return Err(format!("parent {parent_id} has an empty ontology basis"));
                }
            } else if line == "#### Rationale" {
                if !has_children || subsection_order != 2 || rationale_seen {
                    return Err(format!("invalid subsection order for parent {parent_id}"));
                }
                rationale_seen = true;
                cursor += 1;
                while cursor < lines.len()
                    && !lines[cursor].1.starts_with("#### ")
                    && !lines[cursor].1.starts_with("### ")
                    && !lines[cursor].1.starts_with("## ")
                {
                    if !lines[cursor].1.trim().is_empty() {
                        rationale.push(lines[cursor].1);
                    }
                    cursor += 1;
                }
                if rationale.is_empty() {
                    return Err(format!("parent {parent_id} has empty rationale"));
                }
            } else if line.trim().is_empty() {
                cursor += 1;
            } else {
                return Err(format!(
                    "unexpected content at line {}",
                    lines[cursor].0 + 1
                ));
            }
        }
        if !has_children && !has_atomic {
            return Err(format!("parent {parent_id} has no outcome"));
        }
        if has_children && ontology_basis_ids.is_empty() {
            return Err(format!("parent {parent_id} is missing ontology basis"));
        }
        if has_atomic && (!ontology_basis_ids.is_empty() || !rationale.is_empty()) {
            return Err(format!("atomic parent {parent_id} has extra content"));
        }
        let outcome = if has_atomic {
            RequirementDecompositionOutcome::NoFurtherDecomposition
        } else {
            RequirementDecompositionOutcome::Children {
                child_requirement_ids: child_ids,
                ontology_basis_ids,
                rationale: (!rationale.is_empty()).then(|| rationale.join("\n")),
            }
        };
        parents.push(RequirementDecomposition {
            parent_requirement_id: parent_id.to_owned(),
            outcome,
        });
    }
    if parents.is_empty() {
        return Err("decomposition contains no parents".into());
    }
    let mut ids = HashSet::new();
    for parent in &parents {
        if !ids.insert(&parent.parent_requirement_id) {
            return Err(format!(
                "duplicate parent section: {}",
                parent.parent_requirement_id
            ));
        }
    }
    Ok(RequirementDecompositionIndex { parents })
}

fn valid_requirement_id(value: &str) -> bool {
    value
        .strip_prefix("requirement.r")
        .is_some_and(|suffix| suffix.len() == 3 && suffix.bytes().all(|byte| byte.is_ascii_digit()))
}
fn valid_ontology_id(value: &str) -> bool {
    ["concept.c", "property.p", "relationship.r", "constraint.k"]
        .iter()
        .any(|prefix| {
            value.strip_prefix(prefix).is_some_and(|suffix| {
                suffix.len() == 3 && suffix.bytes().all(|byte| byte.is_ascii_digit())
            })
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_children_and_atomic_outcomes() {
        let index = extract_requirement_decomposition(
            "## Decomposition\n\n### `requirement.r001`\n\n#### Child requirements\n\n- `requirement.r002`\n\n#### Ontology basis\n\n- `concept.c001`\n\n#### Rationale\n\nSplits the behavior.\n\n### `requirement.r002`\n\n#### No further decomposition\n",
        ).unwrap();
        assert_eq!(index.parents.len(), 2);
    }

    #[test]
    fn rejects_duplicate_section_and_mixed_outcomes() {
        let duplicate = "## Decomposition\n\n## Decomposition\n";
        assert!(extract_requirement_decomposition(duplicate).is_err());
        let mixed = "## Decomposition\n\n### `requirement.r001`\n\n#### Child requirements\n\n- `requirement.r002`\n\n#### No further decomposition\n";
        assert!(extract_requirement_decomposition(mixed).is_err());
    }

    #[test]
    fn rejects_invalid_managed_subsections() {
        let base = "## Decomposition\n\n### `requirement.r001`\n\n#### Child requirements\n\n- `requirement.r002`\n\n#### Ontology basis\n\n- `concept.c001`\n\n#### Rationale\n\nReason.\n";
        assert!(extract_requirement_decomposition(&base.replace(
            "#### Rationale",
            "#### Ontology basis\n\n- `concept.c002`\n\n#### Rationale"
        ))
        .is_err());
        assert!(extract_requirement_decomposition(&base.replace(
            "#### Rationale\n\nReason.",
            "#### Rationale\n\nReason.\n\n#### Rationale\n\nAgain."
        ))
        .is_err());
        assert!(extract_requirement_decomposition(
            &base.replace("#### Ontology basis\n\n- `concept.c001`\n\n", "")
        )
        .is_err());
    }
}
