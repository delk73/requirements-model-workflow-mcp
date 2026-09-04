use requirements_model_workflow_mcp::{model::CandidateIdentity, store::ModelStore};
use std::{
    collections::BTreeMap,
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

const REQUIREMENTS: &str = "# Requirements\n\n## Requirements\n\n### `requirement.r001`\n\nThe system shall identify captures.\n\n#### Ontology elements\n\n| Ontology element |\n| --- |\n| `concept.c001` |\n";
const IMPLEMENTATION: &str = "# Implementation Targets\n\n## Implementation Targets\n\n| ID | Repository revision | Path | Symbol |\n| --- | --- | --- | --- |\n| `implementation.i001` | 0123456789abcdef0123456789abcdef01234567 | `src/store.rs` | `ModelStore::accept_candidate` |\n";
const TRACE: &str = "# Traceability\n\n## Trace Links\n\n| Source artifact | Source element | Relationship | Target artifact | Target element |\n| --- | --- | --- | --- | --- |\n| raw-adc-requirements | `requirement.r001` | traces_to | raw-adc-domain-ontology | `concept.c001` |\n| raw-adc-domain-ontology | `concept.c001` | refines | raw-adc-requirements | `requirement.r001` |\n| raw-adc-requirements | `requirement.r001` | traces_to | raw-adc-implementation | `implementation.i001` |\n";

fn setup() -> (PathBuf, ModelStore) {
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/raw-adc");
    let dir = std::env::temp_dir().join(format!(
        "rmwm-implementation-{}",
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
    manifest.push_str("\n  raw-adc-traceability-test:\n    type: \"traceability\"\n    representation:\n      path: \"traceability-test.md\"\n      media_type: \"text/markdown\"\n      encoding: \"utf-8\"\n      line_endings: \"lf\"\n    accepted: null\n");
    fs::write(dir.join("requirements_model.yaml"), manifest).unwrap();
    (dir.clone(), ModelStore::open(dir))
}

fn revision(store: &ModelStore, id: &str) -> String {
    store
        .inspect_model_state()
        .unwrap()
        .artifacts
        .into_iter()
        .find(|artifact| artifact.artifact_id == id)
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

fn implementation_identity() -> CandidateIdentity {
    CandidateIdentity {
        model_id: "raw-adc".into(),
        artifact_id: "raw-adc-implementation".into(),
        artifact_type: "implementation".into(),
        target_revision: None,
        source_revisions: BTreeMap::new(),
    }
}

fn trace_identity(
    ontology: String,
    requirements: String,
    implementation: String,
) -> CandidateIdentity {
    CandidateIdentity {
        model_id: "raw-adc".into(),
        artifact_id: "raw-adc-traceability-test".into(),
        artifact_type: "traceability".into(),
        target_revision: None,
        source_revisions: BTreeMap::from([
            ("raw-adc-domain-ontology".into(), ontology),
            ("raw-adc-implementation".into(), implementation),
            ("raw-adc-requirements".into(), requirements),
        ]),
    }
}

#[test]
fn accepts_implementation_and_valid_requirement_trace() {
    let (dir, store, ontology, requirements) = prepare();
    accept(&store, implementation_identity(), IMPLEMENTATION);
    let implementation = revision(&store, "raw-adc-implementation");
    let candidate = store
        .stage_candidate(
            trace_identity(ontology, requirements, implementation.clone()),
            TRACE,
        )
        .unwrap();
    store
        .begin_candidate_review("raw-adc-traceability-test", &candidate.revision)
        .unwrap();
    store
        .record_candidate_decision(
            "raw-adc-traceability-test",
            &candidate.revision,
            "approved",
            "test".into(),
            None,
        )
        .unwrap();
    let accepted = store
        .accept_candidate("raw-adc-traceability-test", &candidate.revision)
        .unwrap();
    assert_eq!(
        accepted.sources.get("raw-adc-implementation"),
        Some(&implementation)
    );
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn rejects_unresolved_implementation_and_missing_binding() {
    let (dir, store, ontology, requirements) = prepare();
    accept(&store, implementation_identity(), IMPLEMENTATION);
    let implementation = revision(&store, "raw-adc-implementation");
    let unresolved = TRACE.replace("implementation.i001", "implementation.i999");
    assert!(store
        .stage_candidate(
            trace_identity(
                ontology.clone(),
                requirements.clone(),
                implementation.clone()
            ),
            &unresolved,
        )
        .unwrap_err()
        .contains("unresolved trace element"));
    let mut missing = trace_identity(ontology, requirements, implementation);
    missing.source_revisions.remove("raw-adc-implementation");
    assert!(store
        .stage_candidate(missing, TRACE)
        .unwrap_err()
        .contains("bindings must match"));
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn rejects_invalid_implementation_revision_and_content_drift() {
    let (dir, store, ontology, requirements) = prepare();
    let mut stale = implementation_identity();
    stale.target_revision = Some("sha256:stale".into());
    assert!(store
        .stage_candidate(stale, IMPLEMENTATION)
        .unwrap_err()
        .contains("stale"));
    accept(&store, implementation_identity(), IMPLEMENTATION);
    let implementation = revision(&store, "raw-adc-implementation");
    fs::write(dir.join("implementation.md"), "drift").unwrap();
    assert!(store
        .stage_candidate(
            trace_identity(ontology, requirements, implementation),
            TRACE,
        )
        .is_err());
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn implementation_reacceptance_marks_traceability_only() {
    let (dir, store, ontology, requirements) = prepare();
    accept(&store, implementation_identity(), IMPLEMENTATION);
    let implementation = revision(&store, "raw-adc-implementation");
    accept(
        &store,
        trace_identity(ontology, requirements, implementation.clone()),
        TRACE,
    );
    let changed = IMPLEMENTATION.replace(
        "ModelStore::accept_candidate",
        "ModelStore::stage_candidate",
    );
    accept(
        &store,
        CandidateIdentity {
            target_revision: Some(implementation),
            ..implementation_identity()
        },
        &changed,
    );
    let state = store.inspect_model_state().unwrap();
    assert_eq!(
        state
            .artifacts
            .iter()
            .find(|artifact| artifact.artifact_id == "raw-adc-traceability-test")
            .unwrap()
            .state,
        "review_required"
    );
    assert_eq!(
        state
            .artifacts
            .iter()
            .find(|artifact| artifact.artifact_id == "raw-adc-requirements")
            .unwrap()
            .state,
        "accepted"
    );
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn stale_implementation_binding_rejects_advancement_and_preserves_candidate() {
    let (dir, store, ontology, requirements) = prepare();
    accept(&store, implementation_identity(), IMPLEMENTATION);
    let implementation_a = revision(&store, "raw-adc-implementation");
    let candidate = store
        .stage_candidate(
            trace_identity(ontology, requirements, implementation_a.clone()),
            TRACE,
        )
        .unwrap();
    store
        .begin_candidate_review("raw-adc-traceability-test", &candidate.revision)
        .unwrap();
    store
        .record_candidate_decision(
            "raw-adc-traceability-test",
            &candidate.revision,
            "approved",
            "test".into(),
            None,
        )
        .unwrap();
    let staged_before =
        fs::read_to_string(dir.join(".rmwm/staged/raw-adc-traceability-test.json")).unwrap();

    let implementation_b = IMPLEMENTATION.replace(
        "ModelStore::accept_candidate",
        "ModelStore::stage_candidate",
    );
    accept(
        &store,
        CandidateIdentity {
            target_revision: Some(implementation_a.clone()),
            ..implementation_identity()
        },
        &implementation_b,
    );

    assert!(store
        .accept_candidate("raw-adc-traceability-test", &candidate.revision)
        .unwrap_err()
        .contains("stale or incorrect source revision"));
    let staged_after =
        fs::read_to_string(dir.join(".rmwm/staged/raw-adc-traceability-test.json")).unwrap();
    assert_eq!(staged_after, staged_before);
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn rejects_implementation_to_requirement_trace() {
    let (dir, store, ontology, requirements) = prepare();
    accept(&store, implementation_identity(), IMPLEMENTATION);
    let implementation = revision(&store, "raw-adc-implementation");
    let invalid = TRACE.replace(
        "raw-adc-requirements | `requirement.r001` | traces_to | raw-adc-implementation | `implementation.i001`",
        "raw-adc-implementation | `implementation.i001` | traces_to | raw-adc-requirements | `requirement.r001`",
    );
    assert!(store
        .stage_candidate(
            trace_identity(ontology, requirements, implementation),
            &invalid,
        )
        .unwrap_err()
        .contains("requirement traces_to implementation"));
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn rejects_refines_requirement_to_implementation_trace() {
    let (dir, store, ontology, requirements) = prepare();
    accept(&store, implementation_identity(), IMPLEMENTATION);
    let implementation = revision(&store, "raw-adc-implementation");
    let invalid = TRACE.replace(
        "raw-adc-requirements | `requirement.r001` | traces_to | raw-adc-implementation | `implementation.i001`",
        "raw-adc-requirements | `requirement.r001` | refines | raw-adc-implementation | `implementation.i001`",
    );
    assert!(store
        .stage_candidate(
            trace_identity(ontology, requirements, implementation),
            &invalid,
        )
        .unwrap_err()
        .contains("requirement traces_to implementation"));
    fs::remove_dir_all(dir).unwrap();
}
