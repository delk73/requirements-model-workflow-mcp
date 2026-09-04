use crate::model::{TraceLink, TraceabilityIndex};
use std::collections::HashSet;

pub fn extract_traceability(body: &str) -> Result<TraceabilityIndex, String> {
    let lines: Vec<_> = body.lines().enumerate().collect();
    let section = lines
        .iter()
        .position(|(_, line)| *line == "## Trace Links")
        .ok_or("missing managed ## Trace Links section")?;
    let mut cursor = section + 1;
    while cursor < lines.len() && lines[cursor].1.trim().is_empty() {
        cursor += 1;
    }
    if cursor >= lines.len() || !lines[cursor].1.starts_with('|') {
        return Err("invalid trace links table header".into());
    }
    let header = cells(lines[cursor].1).ok_or("invalid trace links table header")?;
    if header
        != [
            "Source artifact",
            "Source element",
            "Relationship",
            "Target artifact",
            "Target element",
        ]
    {
        return Err("invalid trace links table header".into());
    }
    cursor += 1;
    if cursor >= lines.len() || !is_separator(lines[cursor].1) {
        return Err("invalid trace links table separator".into());
    }
    cursor += 1;
    let mut links = Vec::new();
    let mut seen = HashSet::new();
    while cursor < lines.len() && lines[cursor].1.starts_with('|') {
        let row = cells(lines[cursor].1).ok_or("trace link row must contain exactly five cells")?;
        if row.len() != 5 || row.iter().any(|cell| cell.is_empty()) {
            return Err(format!(
                "invalid trace link at line {}",
                lines[cursor].0 + 1
            ));
        }
        let link = TraceLink {
            source_artifact_id: row[0].to_owned(),
            source_element_id: unquote(row[1]),
            relationship: unquote(row[2]),
            target_artifact_id: row[3].to_owned(),
            target_element_id: unquote(row[4]),
        };
        if !valid_artifact_id(&link.source_artifact_id)
            || !valid_artifact_id(&link.target_artifact_id)
        {
            return Err("invalid trace artifact ID".into());
        }
        if !valid_element_id(&link.source_element_id) || !valid_element_id(&link.target_element_id)
        {
            return Err(format!(
                "malformed trace element ID at line {}",
                lines[cursor].0 + 1
            ));
        }
        if !matches!(
            link.relationship.as_str(),
            "refines" | "derives_from" | "traces_to"
        ) {
            return Err(format!(
                "unsupported trace relationship: {}",
                link.relationship
            ));
        }
        if !seen.insert(link.clone()) {
            return Err(format!(
                "duplicate trace link at line {}",
                lines[cursor].0 + 1
            ));
        }
        links.push(link);
        cursor += 1;
    }
    if links.is_empty() {
        return Err("traceability candidate contains no trace links".into());
    }
    Ok(TraceabilityIndex { links })
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
        .and_then(|v| v.strip_suffix('`'))
        .unwrap_or(value)
        .to_owned()
}
fn is_separator(line: &str) -> bool {
    cells(line).is_some_and(|row| {
        row.len() == 5
            && row
                .iter()
                .all(|cell| cell.len() >= 3 && cell.bytes().all(|b| b == b'-'))
    })
}
fn valid_artifact_id(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}
fn valid_element_id(value: &str) -> bool {
    [
        "concept.c",
        "property.p",
        "relationship.r",
        "constraint.k",
        "requirement.r",
        "implementation.i",
        "verification.v",
    ]
    .iter()
    .any(|prefix| {
        value
            .strip_prefix(prefix)
            .is_some_and(|s| s.len() == 3 && s.bytes().all(|b| b.is_ascii_digit()))
    })
}

#[cfg(test)]
mod tests {
    use super::extract_traceability;

    const VALID: &str = "## Trace Links\n\n| Source artifact | Source element | Relationship | Target artifact | Target element |\n| --- | --- | --- | --- | --- |\n| requirements | `requirement.r001` | traces_to | ontology | `concept.c001` |\n";

    #[test]
    fn parses_multiple_links() {
        let body = format!(
            "{VALID}| ontology | `concept.c001` | refines | requirements | `requirement.r001` |\n"
        );
        assert_eq!(extract_traceability(&body).unwrap().links.len(), 2);
    }

    #[test]
    fn rejects_missing_or_invalid_table() {
        assert!(extract_traceability("# Traceability").is_err());
        assert!(extract_traceability("## Trace Links\n\n| wrong |\n| --- |\n").is_err());
    }

    #[test]
    fn rejects_unsupported_relationship() {
        assert!(extract_traceability(&VALID.replace("traces_to", "implements")).is_err());
    }

    #[test]
    fn rejects_malformed_element_and_duplicate_links() {
        assert!(
            extract_traceability(&VALID.replace("requirement.r001", "requirement.r01")).is_err()
        );
        let duplicate = format!("{VALID}| requirements | `requirement.r001` | traces_to | ontology | `concept.c001` |\n");
        assert!(extract_traceability(&duplicate).is_err());
    }
}
