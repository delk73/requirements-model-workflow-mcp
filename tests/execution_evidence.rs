use requirements_model_workflow_mcp::{
    markdown_execution_evidence::extract_execution_evidence, model::CandidateIdentity,
    store::ModelStore,
};
use std::{
    collections::BTreeMap,
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

const REV: &str = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const REPO: &str = "033f71de02d73f68ab44fb490a4ea16ba95169de";
const VALID: &str = "# Execution Evidence\n\n## Execution Results\n\nVerification artifact: raw-adc-verification\nVerification revision: sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\nRepository revision: 033f71de02d73f68ab44fb490a4ea16ba95169de\nOccurred at: 2026-09-05T00:00:00Z\nContext: test\n\n| Verification | Outcome |\n| --- | --- |\n| `verification.v001` | passed |\n";

#[test]
fn parses_valid_execution_evidence() {
    assert_eq!(extract_execution_evidence(VALID).unwrap().results.len(), 1);
}

#[test]
fn rejects_malformed_metadata_and_results() {
    for replacement in [
        (REPO, "not-a-revision"),
        ("Context: test", "Context: "),
        ("`verification.v001`", "`verification.v01`"),
        ("passed", "unknown"),
        (
            "| `verification.v001` | passed |\n",
            "| `verification.v001` | passed |\n| `verification.v001` | failed |\n",
        ),
        ("| `verification.v001` | passed |\n", ""),
    ] {
        assert!(extract_execution_evidence(&VALID.replace(replacement.0, replacement.1)).is_err());
    }
    assert!(extract_execution_evidence(
        &VALID.replace("Occurred at: 2026-09-05T00:00:00Z", "Occurred at: ")
    )
    .is_err());
    assert!(
        extract_execution_evidence(&VALID.replace("## Execution Results", "## Other")).is_err()
    );
}

fn setup() -> (PathBuf, ModelStore) {
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/raw-adc");
    let dir = std::env::temp_dir().join(format!(
        "rmwm-execution-evidence-{}",
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
        "verification.md",
    ] {
        fs::copy(source.join(name), dir.join(name)).unwrap();
    }
    let mut manifest = fs::read_to_string(dir.join("requirements_model.yaml")).unwrap();
    for id in [
        "raw-adc-execution-evidence-2",
        "raw-adc-execution-evidence-unresolved",
        "raw-adc-execution-evidence-drift",
    ] {
        manifest.push_str(&format!("\n  {id}:\n    type: \"execution_evidence\"\n    representation:\n      path: \"{id}.md\"\n      media_type: \"text/markdown\"\n      encoding: \"utf-8\"\n      line_endings: \"lf\"\n    accepted: null\n"));
    }
    fs::write(dir.join("requirements_model.yaml"), manifest).unwrap();
    (dir.clone(), ModelStore::open(dir))
}

fn identity(id: &str, verification: &str) -> CandidateIdentity {
    CandidateIdentity {
        model_id: "raw-adc".into(),
        artifact_id: id.into(),
        artifact_type: "execution_evidence".into(),
        target_revision: None,
        source_revisions: BTreeMap::from([("raw-adc-verification".into(), verification.into())]),
    }
}
fn accept(store: &ModelStore, candidate: CandidateIdentity, body: &str) -> String {
    let id = candidate.artifact_id.clone();
    let staged = store.stage_candidate(candidate, body).unwrap();
    store.begin_candidate_review(&id, &staged.revision).unwrap();
    store
        .record_candidate_decision(&id, &staged.revision, "approved", "test".into(), None)
        .unwrap();
    store
        .accept_candidate(&id, &staged.revision)
        .unwrap()
        .revision
}
fn verification(dir: &PathBuf, store: &ModelStore) -> String {
    let state = store.inspect_model_state().unwrap();
    state
        .artifacts
        .into_iter()
        .find(|artifact| artifact.artifact_id == "raw-adc-verification")
        .unwrap()
        .descriptor
        .accepted
        .map(|accepted| accepted.revision)
        .unwrap_or_else(|| {
            let identity = CandidateIdentity {
                model_id: "raw-adc".into(),
                artifact_id: "raw-adc-verification".into(),
                artifact_type: "verification".into(),
                target_revision: None,
                source_revisions: BTreeMap::new(),
            };
            let body = fs::read_to_string(dir.join("verification.md")).unwrap();
            accept(store, identity, &body)
        })
}

#[test]
fn raw_adc_execution_evidence_uses_checked_in_verification_revision() {
    let (dir, store) = setup();
    let verification_revision = verification(&dir, &store);
    let evidence = fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/raw-adc/execution_evidence.md"),
    )
    .unwrap();
    assert_eq!(
        extract_execution_evidence(&evidence)
            .unwrap()
            .verification_revision,
        verification_revision
    );
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn lifecycle_binds_exact_revision_and_preserves_historical_sessions() {
    let (dir, store) = setup();
    let verification_revision = verification(&dir, &store);
    let body = VALID.replace(REV, &verification_revision);
    let evidence_revision = accept(
        &store,
        identity("raw-adc-execution-evidence", &verification_revision),
        &body,
    );
    let original_verification = fs::read_to_string(dir.join("verification.md")).unwrap();
    let accepted = store
        .inspect_model_state()
        .unwrap()
        .artifacts
        .into_iter()
        .find(|a| a.artifact_id == "raw-adc-execution-evidence")
        .unwrap();
    assert_eq!(accepted.state, "accepted");
    assert_eq!(
        accepted.descriptor.accepted.unwrap().sources["raw-adc-verification"],
        verification_revision
    );
    assert!(store
        .stage_candidate(
            identity("raw-adc-execution-evidence", &verification_revision),
            &body
        )
        .is_err());
    let second = accept(
        &store,
        identity("raw-adc-execution-evidence-2", &verification_revision),
        &body,
    );
    assert_ne!(evidence_revision, second);
    let bad_id = body.replace("verification.v001", "verification.v999");
    let unresolved_error = store
        .stage_candidate(
            identity(
                "raw-adc-execution-evidence-unresolved",
                &verification_revision,
            ),
            &bad_id,
        )
        .unwrap_err();
    assert!(
        unresolved_error.contains("unresolved verification ID"),
        "{unresolved_error}"
    );
    fs::write(dir.join("verification.md"), "drift").unwrap();
    let drift_error = store
        .stage_candidate(
            identity("raw-adc-execution-evidence-drift", &verification_revision),
            &body,
        )
        .unwrap_err();
    assert!(drift_error.contains("accepted artifact"), "{drift_error}");
    let revised_verification = "# Verification Targets\n\n## Verification Targets\n\n| ID | Repository revision | Path | Test |\n| --- | --- | --- | --- |\n| `verification.v001` | 033f71de02d73f68ab44fb490a4ea16ba95169de | `tests/execution_evidence.rs` | `lifecycle_binds_exact_revision_and_preserves_historical_sessions` |\n";
    let verification_identity = CandidateIdentity {
        model_id: "raw-adc".into(),
        artifact_id: "raw-adc-verification".into(),
        artifact_type: "verification".into(),
        target_revision: Some(verification_revision.clone()),
        source_revisions: BTreeMap::new(),
    };
    fs::write(dir.join("verification.md"), original_verification).unwrap();
    accept(&store, verification_identity, revised_verification);
    assert_eq!(
        store
            .inspect_model_state()
            .unwrap()
            .artifacts
            .into_iter()
            .find(|a| a.artifact_id == "raw-adc-execution-evidence")
            .unwrap()
            .state,
        "accepted"
    );
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn stale_and_missing_bindings_are_rejected() {
    let (dir, store) = setup();
    let verification_revision = verification(&dir, &store);
    let body = VALID.replace(REV, &verification_revision);
    let mut missing = identity("raw-adc-execution-evidence", &verification_revision);
    missing.source_revisions.clear();
    assert!(store.stage_candidate(missing, &body).is_err());
    let stale = identity(
        "raw-adc-execution-evidence",
        "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
    );
    assert!(store.stage_candidate(stale, &body).is_err());
    fs::remove_dir_all(dir).unwrap();
}
