use requirements_model_workflow_mcp::markdown_verification::extract_verifications;
use requirements_model_workflow_mcp::{model::CandidateIdentity, store::ModelStore};
use std::{
    collections::BTreeMap,
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

const VALID: &str = "## Verification Targets\n\n| ID | Repository revision | Path | Test |\n| --- | --- | --- | --- |\n| `verification.v001` | c3db3b9442ad2ed512748e9fc731bfcb6f26453e | `tests/implementation_traceability.rs` | `stale_implementation_binding_rejects_advancement_and_preserves_candidate` |\n";

#[test]
fn parses_valid_verification_target() {
    assert_eq!(extract_verifications(VALID).unwrap().targets.len(), 1);
}

#[test]
fn rejects_invalid_verification_fields() {
    for replacement in [
        ("verification.v001", "verification.v01"),
        ("c3db3b9442ad2ed512748e9fc731bfcb6f26453e", "not-a-sha"),
        ("`tests/implementation_traceability.rs`", "`/tests/a.rs`"),
        ("`tests/implementation_traceability.rs`", "`../tests/a.rs`"),
        (
            "`stale_implementation_binding_rejects_advancement_and_preserves_candidate`",
            "``",
        ),
    ] {
        assert!(extract_verifications(&VALID.replace(replacement.0, replacement.1)).is_err());
    }
}

#[test]
fn rejects_duplicate_ids_and_targets() {
    let duplicate_id = format!("{VALID}| `verification.v001` | c3db3b9442ad2ed512748e9fc731bfcb6f26453e | `tests/other.rs` | `other` |\n");
    assert!(extract_verifications(&duplicate_id).is_err());
    let duplicate_target = format!("{VALID}| `verification.v002` | c3db3b9442ad2ed512748e9fc731bfcb6f26453e | `tests/implementation_traceability.rs` | `stale_implementation_binding_rejects_advancement_and_preserves_candidate` |\n");
    assert!(extract_verifications(&duplicate_target).is_err());
}

#[test]
fn requires_managed_section_and_target() {
    assert!(extract_verifications("# Verification").is_err());
    assert!(extract_verifications("## Verification Targets\n\n| ID | Repository revision | Path | Test |\n| --- | --- | --- | --- |\n").is_err());
}

const REQUIREMENTS: &str = "# Requirements\n\n## Requirements\n\n### `requirement.r001`\n\nThe system shall identify captures.\n\n#### Ontology elements\n\n| Ontology element |\n| --- |\n| `concept.c001` |\n";
const VERIFICATION: &str = "# Verification Targets\n\n## Verification Targets\n\n| ID | Repository revision | Path | Test |\n| --- | --- | --- | --- |\n| `verification.v001` | c3db3b9442ad2ed512748e9fc731bfcb6f26453e | `tests/implementation_traceability.rs` | `stale_implementation_binding_rejects_advancement_and_preserves_candidate` |\n";
const TRACE: &str = "# Traceability\n\n## Trace Links\n\n| Source artifact | Source element | Relationship | Target artifact | Target element |\n| --- | --- | --- | --- | --- |\n| raw-adc-requirements | `requirement.r001` | traces_to | raw-adc-verification-test | `verification.v001` |\n";

fn setup() -> (PathBuf, ModelStore) {
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/raw-adc");
    let dir = std::env::temp_dir().join(format!(
        "rmwm-verification-{}",
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
    manifest.push_str("\n  raw-adc-verification-test:\n    type: \"verification\"\n    representation:\n      path: \"verification-test.md\"\n      media_type: \"text/markdown\"\n      encoding: \"utf-8\"\n      line_endings: \"lf\"\n    accepted: null\n  raw-adc-traceability-test:\n    type: \"traceability\"\n    representation:\n      path: \"traceability-test.md\"\n      media_type: \"text/markdown\"\n      encoding: \"utf-8\"\n      line_endings: \"lf\"\n    accepted: null\n");
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

fn requirements_identity(vocabulary: String) -> CandidateIdentity {
    CandidateIdentity {
        model_id: "raw-adc".into(),
        artifact_id: "raw-adc-requirements".into(),
        artifact_type: "requirements".into(),
        target_revision: None,
        source_revisions: BTreeMap::from([("raw-adc-controlled-vocabulary".into(), vocabulary)]),
    }
}

fn verification_identity() -> CandidateIdentity {
    CandidateIdentity {
        model_id: "raw-adc".into(),
        artifact_id: "raw-adc-verification-test".into(),
        artifact_type: "verification".into(),
        target_revision: None,
        source_revisions: BTreeMap::new(),
    }
}

fn trace_identity(requirements: String, verification: String) -> CandidateIdentity {
    CandidateIdentity {
        model_id: "raw-adc".into(),
        artifact_id: "raw-adc-traceability-test".into(),
        artifact_type: "traceability".into(),
        target_revision: None,
        source_revisions: BTreeMap::from([
            ("raw-adc-requirements".into(), requirements),
            ("raw-adc-verification-test".into(), verification),
        ]),
    }
}

fn prepared() -> (PathBuf, ModelStore, String) {
    let (dir, store) = setup();
    let vocabulary = revision(&store, "raw-adc-controlled-vocabulary");
    accept(&store, requirements_identity(vocabulary), REQUIREMENTS);
    let requirements = revision(&store, "raw-adc-requirements");
    (dir, store, requirements)
}

#[test]
fn valid_requirement_verification_trace_preserves_exact_binding() {
    let (dir, store, requirements) = prepared();
    accept(&store, verification_identity(), VERIFICATION);
    let verification = revision(&store, "raw-adc-verification-test");
    accept(
        &store,
        trace_identity(requirements, verification.clone()),
        TRACE,
    );
    let accepted = store
        .read_accepted_artifact("raw-adc-traceability-test", None, None)
        .unwrap();
    assert_eq!(
        accepted.descriptor.sources.get("raw-adc-verification-test"),
        Some(&verification)
    );
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn verification_trace_rejects_unresolved_id_and_missing_binding() {
    let (dir, store, requirements) = prepared();
    accept(&store, verification_identity(), VERIFICATION);
    let verification = revision(&store, "raw-adc-verification-test");
    assert!(store
        .stage_candidate(
            trace_identity(requirements.clone(), verification.clone()),
            &TRACE.replace("verification.v001", "verification.v999")
        )
        .unwrap_err()
        .contains("unresolved trace element"));
    let mut missing = trace_identity(requirements, verification);
    missing.source_revisions.remove("raw-adc-verification-test");
    assert!(store
        .stage_candidate(missing, TRACE)
        .unwrap_err()
        .contains("bindings must match"));
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn stale_or_drifted_verification_source_is_rejected() {
    let (dir, store, requirements) = prepared();
    accept(&store, verification_identity(), VERIFICATION);
    let verification = revision(&store, "raw-adc-verification-test");
    let mut stale = trace_identity(requirements.clone(), verification.clone());
    stale
        .source_revisions
        .insert("raw-adc-verification-test".into(), "sha256:stale".into());
    assert!(store
        .stage_candidate(stale, TRACE)
        .unwrap_err()
        .contains("stale"));
    fs::write(dir.join("verification-test.md"), "drift").unwrap();
    assert!(store
        .stage_candidate(trace_identity(requirements, verification), TRACE)
        .is_err());
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn stale_verification_binding_rejects_acceptance_and_preserves_candidate() {
    let (dir, store, requirements) = prepared();
    accept(&store, verification_identity(), VERIFICATION);
    let verification_a = revision(&store, "raw-adc-verification-test");
    let candidate = store
        .stage_candidate(trace_identity(requirements, verification_a.clone()), TRACE)
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

    let changed = VERIFICATION.replace(
        "stale_implementation_binding_rejects_advancement_and_preserves_candidate",
        "parses_valid_verification_target",
    );
    accept(
        &store,
        CandidateIdentity {
            target_revision: Some(verification_a),
            ..verification_identity()
        },
        &changed,
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
fn verification_reacceptance_marks_traceability_only() {
    let (dir, store, requirements) = prepared();
    accept(&store, verification_identity(), VERIFICATION);
    let verification = revision(&store, "raw-adc-verification-test");
    accept(
        &store,
        trace_identity(requirements, verification.clone()),
        TRACE,
    );
    let changed = VERIFICATION.replace(
        "stale_implementation_binding_rejects_advancement_and_preserves_candidate",
        "parses_valid_verification_target",
    );
    accept(
        &store,
        CandidateIdentity {
            target_revision: Some(verification),
            ..verification_identity()
        },
        &changed,
    );
    let state = store.inspect_model_state().unwrap();
    assert_eq!(
        state
            .artifacts
            .iter()
            .find(|a| a.artifact_id == "raw-adc-traceability-test")
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

#[test]
fn verification_links_only_allow_requirement_traces_to_verification() {
    let (dir, store, requirements) = prepared();
    accept(&store, verification_identity(), VERIFICATION);
    let verification = revision(&store, "raw-adc-verification-test");
    for invalid in [
        TRACE.replace(
            "raw-adc-requirements | `requirement.r001` | traces_to | raw-adc-verification-test | `verification.v001`",
            "raw-adc-verification-test | `verification.v001` | traces_to | raw-adc-requirements | `requirement.r001`",
        ),
        TRACE.replace(
            "raw-adc-requirements | `requirement.r001` | traces_to",
            "raw-adc-requirements | `requirement.r001` | refines",
        ),
    ] {
        assert!(store
            .stage_candidate(
                trace_identity(requirements.clone(), verification.clone()),
                &invalid
            )
            .is_err());
    }
    fs::remove_dir_all(dir).unwrap();
}
