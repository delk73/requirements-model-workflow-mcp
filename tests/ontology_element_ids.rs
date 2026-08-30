use requirements_model_workflow_mcp::{
    markdown_ontology::extract_ontology_elements, model::CandidateIdentity, store::ModelStore,
};
use std::{
    collections::BTreeMap,
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

const VALID_ONTOLOGY: &str = "# Ontology\n\n## Concepts\n\n| ID | Concept |\n| --- | --- |\n| `concept.c001` | Capture |\n\n## Properties\n\n| ID | Property |\n| --- | --- |\n| `property.p001` | Capture identity |\n\n## Relationships\n\n| ID | Relationship |\n| --- | --- |\n| `relationship.r001` | relates to |\n\n## Constraints\n\n* `constraint.k001` A constraint.\n";

fn fixture() -> (PathBuf, ModelStore, CandidateIdentity) {
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/raw-adc");
    let dir = std::env::temp_dir().join(format!(
        "rmwm-ontology-ids-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&dir).unwrap();
    for name in [
        "requirements_model.yaml",
        "story.md",
        "domain_framing.md",
        "domain_ontology.md",
    ] {
        fs::copy(source.join(name), dir.join(name)).unwrap();
    }
    let store = ModelStore::open(&dir);
    let state = store.inspect_model_state().unwrap();
    let revision = |artifact_id: &str| {
        state
            .artifacts
            .iter()
            .find(|artifact| artifact.artifact_id == artifact_id)
            .unwrap()
            .descriptor
            .accepted
            .as_ref()
            .unwrap()
            .revision
            .clone()
    };
    let identity = CandidateIdentity {
        model_id: "raw-adc".into(),
        artifact_id: "raw-adc-domain-ontology".into(),
        artifact_type: "domain_ontology".into(),
        target_revision: Some(revision("raw-adc-domain-ontology")),
        source_revisions: BTreeMap::from([(
            "raw-adc-domain-framing".into(),
            revision("raw-adc-domain-framing"),
        )]),
    };
    (dir, store, identity)
}

#[test]
fn extracts_complete_typed_ontology_ids() {
    let index = extract_ontology_elements(VALID_ONTOLOGY).unwrap();
    assert_eq!(index.elements.len(), 4);
}

#[test]
fn staging_requires_complete_valid_and_unique_ontology_ids() {
    let cases = [
        (
            VALID_ONTOLOGY.replace("`concept.c001`", ""),
            "missing concepts ID",
        ),
        (
            VALID_ONTOLOGY.replace("concept.c001", "property.p001"),
            "invalid concepts ID",
        ),
        (
            VALID_ONTOLOGY.replace("concept.c001", "concept.c01"),
            "invalid concepts ID",
        ),
        (
            VALID_ONTOLOGY.replace("`constraint.k001`", "A constraint"),
            "invalid constraints ID",
        ),
        (
            VALID_ONTOLOGY.replace("relationship.r001", "concept.c001"),
            "invalid relationships ID",
        ),
        (
            VALID_ONTOLOGY.replace(
                "* `constraint.k001` A constraint.",
                "* `constraint.k001` A constraint.\n* `constraint.k001` Another constraint.",
            ),
            "duplicate ontology element ID",
        ),
    ];
    for (body, expected) in cases {
        let (dir, store, identity) = fixture();
        let error = store.stage_candidate(identity, &body).unwrap_err();
        assert!(
            error.contains(expected),
            "expected {expected:?}, got {error:?}"
        );
        fs::remove_dir_all(dir).unwrap();
    }
}

#[test]
fn staging_rejects_an_ontology_without_managed_elements() {
    let (dir, store, identity) = fixture();
    assert_eq!(
        store.stage_candidate(identity, "# Ontology").unwrap_err(),
        "ontology candidate contains no managed ontology elements"
    );
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn accepted_legacy_ontology_without_ids_remains_readable() {
    let (dir, store, _identity) = fixture();
    assert!(store
        .read_accepted_artifact("raw-adc-domain-ontology", None, None)
        .is_ok());
    fs::remove_dir_all(dir).unwrap();
}
