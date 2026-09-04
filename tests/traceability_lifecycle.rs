use requirements_model_workflow_mcp::{model::CandidateIdentity, store::ModelStore};
use std::{
    collections::BTreeMap,
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

const REQUIREMENTS: &str = "# Requirements\n\n## Requirements\n\n### `requirement.r001`\n\nThe system shall identify captures.\n\n#### Ontology elements\n\n| Ontology element |\n| --- |\n| `concept.c001` |\n";
const TRACE: &str = "# Traceability\n\n## Trace Links\n\n| Source artifact | Source element | Relationship | Target artifact | Target element |\n| --- | --- | --- | --- | --- |\n| raw-adc-requirements | `requirement.r001` | traces_to | raw-adc-domain-ontology | `concept.c001` |\n| raw-adc-domain-ontology | `concept.c001` | refines | raw-adc-requirements | `requirement.r001` |\n";

fn setup() -> (PathBuf, ModelStore) {
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/raw-adc");
    let dir = std::env::temp_dir().join(format!(
        "rmwm-trace-{}",
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
        "controlled_vocabulary.md",
    ] {
        fs::copy(source.join(name), dir.join(name)).unwrap();
    }
    let mut manifest = fs::read_to_string(dir.join("requirements_model.yaml")).unwrap();
    manifest.push_str("\n  traceability:\n    type: \"traceability\"\n    representation:\n      path: \"traceability.md\"\n      media_type: \"text/markdown\"\n      encoding: \"utf-8\"\n      line_endings: \"lf\"\n    accepted: null\n");
    fs::write(dir.join("requirements_model.yaml"), manifest).unwrap();
    (dir.clone(), ModelStore::open(dir))
}
fn revision(store: &ModelStore, id: &str) -> String {
    store
        .inspect_model_state()
        .unwrap()
        .artifacts
        .into_iter()
        .find(|a| a.artifact_id == id)
        .unwrap()
        .descriptor
        .accepted
        .unwrap()
        .revision
}
fn accept(store: &ModelStore, identity: CandidateIdentity, body: &str) {
    let id = identity.artifact_id.clone();
    let candidate = store.stage_candidate(identity, body).unwrap();
    store
        .begin_candidate_review(&id, &candidate.revision)
        .unwrap();
    store
        .record_candidate_decision(&id, &candidate.revision, "approved", "test".into(), None)
        .unwrap();
    store.accept_candidate(&id, &candidate.revision).unwrap();
}
fn prepare() -> (PathBuf, ModelStore, String, String) {
    let (dir, store) = setup();
    let vocabulary = revision(&store, "raw-adc-controlled-vocabulary");
    accept(
        &store,
        CandidateIdentity {
            model_id: "raw-adc".into(),
            artifact_id: "raw-adc-requirements".into(),
            artifact_type: "requirements".into(),
            target_revision: None,
            source_revisions: BTreeMap::from([(
                "raw-adc-controlled-vocabulary".into(),
                vocabulary,
            )]),
        },
        REQUIREMENTS,
    );
    let ontology = revision(&store, "raw-adc-domain-ontology");
    let requirements = revision(&store, "raw-adc-requirements");
    (dir, store, ontology, requirements)
}
fn trace_identity(ontology: String, requirements: String) -> CandidateIdentity {
    CandidateIdentity {
        model_id: "raw-adc".into(),
        artifact_id: "traceability".into(),
        artifact_type: "traceability".into(),
        target_revision: None,
        source_revisions: BTreeMap::from([
            ("raw-adc-domain-ontology".into(), ontology),
            ("raw-adc-requirements".into(), requirements),
        ]),
    }
}

#[test]
fn valid_bidirectional_links_and_exact_sources_are_accepted() {
    let (dir, store, ontology, requirements) = prepare();
    let candidate = store
        .stage_candidate(
            trace_identity(ontology.clone(), requirements.clone()),
            TRACE,
        )
        .unwrap();
    store
        .begin_candidate_review("traceability", &candidate.revision)
        .unwrap();
    store
        .record_candidate_decision(
            "traceability",
            &candidate.revision,
            "approved",
            "test".into(),
            None,
        )
        .unwrap();
    let accepted = store
        .accept_candidate("traceability", &candidate.revision)
        .unwrap();
    assert_eq!(
        accepted.sources.get("raw-adc-domain-ontology"),
        Some(&ontology)
    );
    assert_eq!(
        accepted.sources.get("raw-adc-requirements"),
        Some(&requirements)
    );
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn trace_links_reject_unresolved_elements_and_binding_mismatch() {
    let cases = [
        ("requirement.r999", "unresolved trace element"),
        ("concept.c999", "unresolved trace element"),
    ];
    for (element, error) in cases {
        let (dir, store, ontology, requirements) = prepare();
        let body = if element.starts_with("requirement") {
            TRACE.replace("requirement.r001", element)
        } else {
            TRACE.replace("concept.c001", element)
        };
        assert!(store
            .stage_candidate(trace_identity(ontology, requirements), &body)
            .unwrap_err()
            .contains(error));
        fs::remove_dir_all(dir).unwrap();
    }
    let (dir, store, ontology, requirements) = prepare();
    let mut identity = trace_identity(ontology, requirements);
    identity.source_revisions.remove("raw-adc-requirements");
    assert!(store
        .stage_candidate(identity, TRACE)
        .unwrap_err()
        .contains("bindings must match"));
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn stale_revision_and_content_drift_are_rejected() {
    let (dir, store, ontology, requirements) = prepare();
    let mut stale = trace_identity(ontology, requirements);
    stale
        .source_revisions
        .insert("raw-adc-requirements".into(), "sha256:stale".into());
    assert!(store
        .stage_candidate(stale, TRACE)
        .unwrap_err()
        .contains("stale"));
    fs::write(dir.join("requirements.md"), b"drift").unwrap();
    assert!(store
        .stage_candidate(
            trace_identity(
                revision(&store, "raw-adc-domain-ontology"),
                revision(&store, "raw-adc-requirements")
            ),
            TRACE
        )
        .is_err());
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn direct_reacceptance_marks_trace_only() {
    let (dir, store, ontology, requirements) = prepare();
    accept(
        &store,
        trace_identity(ontology.clone(), requirements),
        TRACE,
    );
    let framing = revision(&store, "raw-adc-domain-framing");
    let changed = fs::read_to_string(dir.join("domain_ontology.md"))
        .unwrap()
        .split_once("---\n\n")
        .unwrap()
        .1
        .to_owned()
        + "\n";
    accept(
        &store,
        CandidateIdentity {
            model_id: "raw-adc".into(),
            artifact_id: "raw-adc-domain-ontology".into(),
            artifact_type: "domain_ontology".into(),
            target_revision: Some(ontology),
            source_revisions: BTreeMap::from([("raw-adc-domain-framing".into(), framing)]),
        },
        &changed,
    );
    let state = store.inspect_model_state().unwrap();
    assert_eq!(
        state
            .artifacts
            .iter()
            .find(|a| a.artifact_id == "raw-adc-controlled-vocabulary")
            .unwrap()
            .state,
        "review_required"
    );
    assert_eq!(
        state
            .artifacts
            .iter()
            .find(|a| a.artifact_id == "traceability")
            .unwrap()
            .state,
        "review_required"
    );
    assert_eq!(
        state
            .artifacts
            .iter()
            .find(|a| a.artifact_id == "raw-adc-requirements")
            .unwrap()
            .state,
        "accepted"
    );
    fs::remove_dir_all(dir).unwrap();
}
