use requirements_model_workflow_mcp::markdown_requirements::extract_requirements;

const BODY: &str = "# Requirements\n\n## Requirements\n\n### `requirement.r001`\n\nThe system shall identify each capture using a stable capture identity.\n\n#### Ontology elements\n\n| Ontology element |\n| --- |\n| `concept.c001` |\n| `property.p001` |\n";

#[test]
fn extracts_requirement_with_multiple_references() {
    let index = extract_requirements(BODY).unwrap();
    assert_eq!(index.requirements.len(), 1);
    assert_eq!(index.requirements[0].id, "requirement.r001");
    assert_eq!(
        index.requirements[0].ontology_element_ids,
        ["concept.c001", "property.p001"]
    );
}

#[test]
fn rejects_duplicate_reference_in_requirement() {
    let body = BODY.replace("| `property.p001` |", "| `concept.c001` |");
    let error = extract_requirements(&body).unwrap_err();
    assert!(error.contains("duplicate ontology reference"));
}

#[test]
fn rejects_empty_prose() {
    let body = BODY.replace(
        "The system shall identify each capture using a stable capture identity.\n",
        "",
    );
    let error = extract_requirements(&body).unwrap_err();
    assert!(error.contains("empty normative prose"));
}

#[test]
fn rejects_extra_separator_column() {
    let body = BODY.replace("| --- |", "| --- | --- |");
    let error = extract_requirements(&body).unwrap_err();
    assert!(error.contains("invalid Markdown table separator"));
}

#[test]
fn rejects_extra_data_column() {
    let body = BODY.replace("| `concept.c001` |", "| `concept.c001` | extra |");
    let error = extract_requirements(&body).unwrap_err();
    assert!(error.contains("exactly one cell"));
}

#[test]
fn rejects_malformed_separator() {
    let body = BODY.replace("| --- |", "| :-: |");
    let error = extract_requirements(&body).unwrap_err();
    assert!(error.contains("invalid Markdown table separator"));
}

#[test]
fn rejects_duplicate_requirements_section() {
    let body = format!("{BODY}\n## Requirements\n");
    let error = extract_requirements(&body).unwrap_err();
    assert!(error.contains("duplicate ## Requirements section"));
}
