use requirements_model_workflow_mcp::{model::CandidateIdentity, store::ModelStore};
use std::{
    collections::BTreeMap,
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

const ONTOLOGY: &str = "# Ontology\n\n## Concepts\n\n| ID | Concept |\n| --- | --- |\n| `concept.c001` | Capture |\n| `concept.c002` | Other |\n";
const VOCABULARY: &str = "# Vocabulary\n\n## Entries\n\n| Ontology element | Preferred term | Definition |\n| --- | --- | --- |\n| `concept.c001` | Capture | A capture. |\n";
const REQUIREMENTS: &str = "# Requirements\n\n## Requirements\n\n### `requirement.r001`\n\nThe system shall identify captures.\n\n#### Ontology elements\n\n| Ontology element |\n| --- |\n| `concept.c001` |\n";

fn setup() -> (PathBuf, ModelStore, String, String) {
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/raw-adc");
    let dir = std::env::temp_dir().join(format!(
        "rmwm-requirements-{}",
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
    let mut manifest = fs::read_to_string(dir.join("requirements_model.yaml")).unwrap();
    manifest.push_str("\n  vocabulary:\n    type: \"controlled_vocabulary\"\n    representation:\n      path: \"vocabulary.md\"\n      media_type: \"text/markdown\"\n      encoding: \"utf-8\"\n      line_endings: \"lf\"\n    accepted: null\n  requirements:\n    type: \"requirements\"\n    representation:\n      path: \"requirements.md\"\n      media_type: \"text/markdown\"\n      encoding: \"utf-8\"\n      line_endings: \"lf\"\n    accepted: null\n");
    fs::write(dir.join("requirements_model.yaml"), manifest).unwrap();
    let store = ModelStore::open(&dir);
    let state = store.inspect_model_state().unwrap();
    let revision = |id: &str| {
        state
            .artifacts
            .iter()
            .find(|artifact| artifact.artifact_id == id)
            .unwrap()
            .descriptor
            .accepted
            .as_ref()
            .unwrap()
            .revision
            .clone()
    };
    (
        dir,
        store,
        revision("raw-adc-domain-framing"),
        revision("raw-adc-domain-ontology"),
    )
}
fn identity(
    id: &str,
    artifact_type: &str,
    target: Option<String>,
    source: (&str, String),
) -> CandidateIdentity {
    CandidateIdentity {
        model_id: "raw-adc".into(),
        artifact_id: id.into(),
        artifact_type: artifact_type.into(),
        target_revision: target,
        source_revisions: BTreeMap::from([(source.0.into(), source.1)]),
    }
}
fn accept(store: &ModelStore, candidate: CandidateIdentity, body: &str) {
    let id = candidate.artifact_id.clone();
    let staged = store.stage_candidate(candidate, body).unwrap();
    store.begin_candidate_review(&id, &staged.revision).unwrap();
    store
        .record_candidate_decision(&id, &staged.revision, "approved", "test".into(), None)
        .unwrap();
    store.accept_candidate(&id, &staged.revision).unwrap();
}

#[test]
fn requirements_staging_validates_source_and_admitted_references() {
    let (dir, store, framing, ontology) = setup();
    let ontology_id = identity(
        "raw-adc-domain-ontology",
        "domain_ontology",
        Some(ontology),
        ("raw-adc-domain-framing", framing),
    );
    accept(&store, ontology_id, ONTOLOGY);
    let vocabulary_id = identity(
        "vocabulary",
        "controlled_vocabulary",
        None,
        (
            "raw-adc-domain-ontology",
            store
                .inspect_model_state()
                .unwrap()
                .artifacts
                .iter()
                .find(|a| a.artifact_id == "raw-adc-domain-ontology")
                .unwrap()
                .descriptor
                .accepted
                .as_ref()
                .unwrap()
                .revision
                .clone(),
        ),
    );
    accept(&store, vocabulary_id, VOCABULARY);
    let vocabulary_revision = store
        .inspect_model_state()
        .unwrap()
        .artifacts
        .iter()
        .find(|a| a.artifact_id == "vocabulary")
        .unwrap()
        .descriptor
        .accepted
        .as_ref()
        .unwrap()
        .revision
        .clone();
    let requirements = identity(
        "requirements",
        "requirements",
        None,
        ("vocabulary", vocabulary_revision.clone()),
    );
    store.begin_candidate(requirements.clone()).unwrap();
    assert!(store
        .stage_candidate(
            requirements.clone(),
            &REQUIREMENTS.replace("concept.c001", "concept.c002")
        )
        .unwrap_err()
        .contains("not admitted"));
    assert!(store
        .stage_candidate(
            requirements.clone(),
            &REQUIREMENTS.replace("concept.c001", "concept.c01")
        )
        .unwrap_err()
        .contains("malformed"));
    assert!(store
        .stage_candidate(
            requirements.clone(),
            &REQUIREMENTS.replace("| `concept.c001` |\n", "")
        )
        .unwrap_err()
        .contains("no ontology references"));
    assert!(store
        .stage_candidate(
            requirements.clone(),
            &format!("{REQUIREMENTS}\n### `requirement.r001`\n\nAnother obligation.\n\n#### Ontology elements\n\n| Ontology element |\n| --- |\n| `concept.c001` |\n")
        )
        .unwrap_err()
        .contains("duplicate requirement ID"));
    let mut wrong = requirements.clone();
    wrong.source_revisions = BTreeMap::from([(
        "raw-adc-domain-ontology".into(),
        vocabulary_revision.clone(),
    )]);
    assert!(store
        .begin_candidate(wrong)
        .unwrap_err()
        .contains("controlled vocabulary"));
    let mut multiple = requirements.clone();
    multiple
        .source_revisions
        .insert("raw-adc-domain-framing".into(), "x".into());
    assert!(store
        .begin_candidate(multiple)
        .unwrap_err()
        .contains("exactly one source"));
    let mut stale = requirements.clone();
    stale
        .source_revisions
        .insert("vocabulary".into(), "sha256:stale".into());
    assert!(store.begin_candidate(stale).unwrap_err().contains("stale"));
    accept(&store, requirements, REQUIREMENTS);
    let vocabulary_target = Some(vocabulary_revision);
    let ontology_revision = store
        .inspect_model_state()
        .unwrap()
        .artifacts
        .iter()
        .find(|a| a.artifact_id == "raw-adc-domain-ontology")
        .unwrap()
        .descriptor
        .accepted
        .as_ref()
        .unwrap()
        .revision
        .clone();
    accept(
        &store,
        identity(
            "vocabulary",
            "controlled_vocabulary",
            vocabulary_target,
            ("raw-adc-domain-ontology", ontology_revision),
        ),
        &format!("{VOCABULARY}\n"),
    );
    assert_eq!(
        store
            .inspect_model_state()
            .unwrap()
            .artifacts
            .iter()
            .find(|a| a.artifact_id == "requirements")
            .unwrap()
            .state,
        "review_required"
    );
    fs::remove_dir_all(dir).unwrap();
}
