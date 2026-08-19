use requirements_model_workflow_mcp::{model::CandidateIdentity, store::ModelStore};
use std::{
    collections::BTreeMap,
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

fn fixture() -> (PathBuf, ModelStore, String) {
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/raw-adc");
    let dir = std::env::temp_dir().join(format!(
        "rmwm-review-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&dir).unwrap();
    for name in ["requirements_model.yaml", "story.md"] {
        fs::copy(source.join(name), dir.join(name)).unwrap();
    }
    let mut manifest = fs::read_to_string(dir.join("requirements_model.yaml")).unwrap();
    let start = manifest.find("  raw-adc-domain-framing:").unwrap();
    let end = manifest[start..]
        .find("\n\n  raw-adc-domain-ontology:")
        .unwrap()
        + start;
    manifest.replace_range(start..end, "  raw-adc-domain-framing:\n    type: \"domain_framing\"\n    representation:\n      path: \"domain_framing.md\"\n      media_type: \"text/markdown\"\n      encoding: \"utf-8\"\n      line_endings: \"lf\"\n    accepted: null");
    fs::write(dir.join("requirements_model.yaml"), manifest).unwrap();
    let store = ModelStore::open(&dir);
    let revision = store
        .inspect_model_state()
        .unwrap()
        .artifacts
        .iter()
        .find(|artifact| artifact.artifact_id == "raw-adc-story")
        .unwrap()
        .descriptor
        .accepted
        .as_ref()
        .unwrap()
        .revision
        .clone();
    (dir, store, revision)
}

fn identity(story_revision: String) -> CandidateIdentity {
    CandidateIdentity {
        model_id: "raw-adc".into(),
        artifact_id: "raw-adc-domain-framing".into(),
        artifact_type: "domain_framing".into(),
        target_revision: None,
        source_revisions: BTreeMap::from([("raw-adc-story".into(), story_revision)]),
    }
}

fn stage(
    store: &ModelStore,
    story_revision: String,
) -> requirements_model_workflow_mcp::model::StagedCandidate {
    store
        .stage_candidate(identity(story_revision), "# Framing")
        .unwrap()
}

#[test]
fn exact_staged_candidate_can_be_read_and_reviewed_without_mutating_inputs() {
    let (dir, store, revision) = fixture();
    let manifest = fs::read(dir.join("requirements_model.yaml")).unwrap();
    let story = fs::read(dir.join("story.md")).unwrap();
    let staged = stage(&store, revision);
    let staged_path = dir.join(".rmwm/staged/raw-adc-domain-framing.json");
    let staged_bytes = fs::read(&staged_path).unwrap();
    assert_eq!(
        store
            .read_staged_candidate("raw-adc-domain-framing")
            .unwrap()
            .bytes,
        staged.bytes
    );
    let request = store
        .begin_candidate_review("raw-adc-domain-framing", &staged.revision)
        .unwrap();
    assert_eq!(request.candidate_revision, staged.revision);
    assert_eq!(
        store
            .inspect_model_state()
            .unwrap()
            .artifacts
            .iter()
            .find(|artifact| artifact.artifact_id == "raw-adc-domain-framing")
            .unwrap()
            .state,
        "under_review"
    );
    assert_eq!(fs::read(&staged_path).unwrap(), staged_bytes);
    assert_eq!(
        fs::read(dir.join("requirements_model.yaml")).unwrap(),
        manifest
    );
    assert_eq!(fs::read(dir.join("story.md")).unwrap(), story);
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn altered_staged_record_and_revision_mismatch_are_rejected() {
    let (dir, store, revision) = fixture();
    let staged = stage(&store, revision);
    assert!(store
        .begin_candidate_review("raw-adc-domain-framing", "sha256:wrong")
        .is_err());
    let path = dir.join(".rmwm/staged/raw-adc-domain-framing.json");
    let mut value: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    value["content"]["size"] = serde_json::json!(0);
    fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
    assert!(store
        .read_staged_candidate("raw-adc-domain-framing")
        .is_err());
    assert!(store
        .begin_candidate_review("raw-adc-domain-framing", &staged.revision)
        .is_err());
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn approval_rejection_and_duplicate_decisions_are_immutable() {
    let (dir, store, revision) = fixture();
    let staged = stage(&store, revision);
    assert!(store
        .record_candidate_decision(
            "raw-adc-domain-framing",
            &staged.revision,
            "approved",
            "reviewer".into(),
            None
        )
        .is_err());
    store
        .begin_candidate_review("raw-adc-domain-framing", &staged.revision)
        .unwrap();
    let decision = store
        .record_candidate_decision(
            "raw-adc-domain-framing",
            &staged.revision,
            "approved",
            "reviewer".into(),
            Some("looks good".into()),
        )
        .unwrap();
    assert_eq!(decision.decision, "approved");
    assert_eq!(
        store
            .inspect_model_state()
            .unwrap()
            .artifacts
            .iter()
            .find(|artifact| artifact.artifact_id == "raw-adc-domain-framing")
            .unwrap()
            .state,
        "approved"
    );
    assert!(store
        .record_candidate_decision(
            "raw-adc-domain-framing",
            &staged.revision,
            "rejected",
            "reviewer".into(),
            None
        )
        .is_err());
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn rejection_produces_rejected_state() {
    let (dir, store, revision) = fixture();
    let staged = stage(&store, revision);
    store
        .begin_candidate_review("raw-adc-domain-framing", &staged.revision)
        .unwrap();
    store
        .record_candidate_decision(
            "raw-adc-domain-framing",
            &staged.revision,
            "rejected",
            "reviewer".into(),
            None,
        )
        .unwrap();
    assert_eq!(
        store
            .inspect_model_state()
            .unwrap()
            .artifacts
            .iter()
            .find(|artifact| artifact.artifact_id == "raw-adc-domain-framing")
            .unwrap()
            .state,
        "rejected"
    );
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn stale_target_or_source_prevents_review() {
    let (dir, store, revision) = fixture();
    let staged = stage(&store, revision);
    let mut manifest = fs::read_to_string(dir.join("requirements_model.yaml")).unwrap();
    manifest = manifest.replace("accepted: null", "accepted:\n      revision: \"sha256:new-target\"\n      content:\n        digest: \"sha256:x\"\n        size: 1\n      sources: {}");
    fs::write(dir.join("requirements_model.yaml"), manifest).unwrap();
    assert!(store
        .begin_candidate_review("raw-adc-domain-framing", &staged.revision)
        .is_err());
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn stale_source_revision_prevents_review() {
    let (dir, store, revision) = fixture();
    let staged = stage(&store, revision);
    let manifest_path = dir.join("requirements_model.yaml");
    let manifest = fs::read_to_string(&manifest_path).unwrap().replace(
        "sha256:d9fc45a0fae8dccf8c4a6ddc7f13d1c4604775b0d1f03abfa92d8f4ec1ffe0ae",
        "sha256:updated-story-revision",
    );
    fs::write(manifest_path, manifest).unwrap();
    assert!(store
        .begin_candidate_review("raw-adc-domain-framing", &staged.revision)
        .is_err());
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn modified_source_file_prevents_decision() {
    let (dir, store, revision) = fixture();
    let staged = stage(&store, revision);
    store
        .begin_candidate_review("raw-adc-domain-framing", &staged.revision)
        .unwrap();
    fs::write(dir.join("story.md"), b"modified").unwrap();
    assert!(store
        .record_candidate_decision(
            "raw-adc-domain-framing",
            &staged.revision,
            "approved",
            "reviewer".into(),
            None
        )
        .is_err());
    fs::remove_dir_all(dir).unwrap();
}

#[cfg(unix)]
#[test]
fn symlinked_review_path_is_rejected_without_external_mutation() {
    use std::os::unix::fs::symlink;
    let (dir, store, revision) = fixture();
    let staged = stage(&store, revision);
    let outside = dir.parent().unwrap().join("rmwm-review-outside");
    fs::create_dir_all(&outside).unwrap();
    symlink(&outside, dir.join(".rmwm/reviews")).unwrap();
    assert!(store
        .begin_candidate_review("raw-adc-domain-framing", &staged.revision)
        .is_err());
    assert!(fs::read_dir(&outside).unwrap().next().is_none());
    fs::remove_file(dir.join(".rmwm/reviews")).unwrap();
    fs::remove_dir_all(&outside).unwrap();
    fs::remove_dir_all(dir).unwrap();
}

#[cfg(unix)]
#[test]
fn symlinked_staged_record_is_rejected_without_reading_external_bytes() {
    use std::os::unix::fs::symlink;
    let (dir, store, revision) = fixture();
    let staged = stage(&store, revision);
    let outside = dir
        .parent()
        .unwrap()
        .join("rmwm-staged-record-outside.json");
    let original = fs::read(dir.join(".rmwm/staged/raw-adc-domain-framing.json")).unwrap();
    fs::write(&outside, &original).unwrap();
    fs::remove_file(dir.join(".rmwm/staged/raw-adc-domain-framing.json")).unwrap();
    symlink(
        &outside,
        dir.join(".rmwm/staged/raw-adc-domain-framing.json"),
    )
    .unwrap();
    assert!(store
        .read_staged_candidate("raw-adc-domain-framing")
        .is_err());
    assert_eq!(fs::read(&outside).unwrap(), original);
    assert!(store
        .begin_candidate_review("raw-adc-domain-framing", &staged.revision)
        .is_err());
    fs::remove_file(dir.join(".rmwm/staged/raw-adc-domain-framing.json")).unwrap();
    fs::remove_file(outside).unwrap();
    fs::remove_dir_all(dir).unwrap();
}

#[cfg(unix)]
#[test]
fn symlinked_request_and_decision_records_are_rejected_without_external_mutation() {
    use std::os::unix::fs::symlink;
    let (dir, store, revision) = fixture();
    let staged = stage(&store, revision);
    store
        .begin_candidate_review("raw-adc-domain-framing", &staged.revision)
        .unwrap();
    let record_dir = dir.join(".rmwm/reviews/raw-adc-domain-framing");
    let outside_request = dir.parent().unwrap().join("rmwm-request-outside.json");
    fs::write(&outside_request, b"request").unwrap();
    let request_path = record_dir.join(format!("{}.request.json", staged.revision));
    fs::remove_file(&request_path).unwrap();
    symlink(&outside_request, &request_path).unwrap();
    assert!(store
        .record_candidate_decision(
            "raw-adc-domain-framing",
            &staged.revision,
            "approved",
            "reviewer".into(),
            None
        )
        .is_err());
    assert_eq!(fs::read(&outside_request).unwrap(), b"request");
    fs::remove_file(&request_path).unwrap();
    store
        .begin_candidate_review("raw-adc-domain-framing", &staged.revision)
        .unwrap();
    let outside_decision = dir.parent().unwrap().join("rmwm-decision-outside.json");
    fs::write(&outside_decision, b"decision").unwrap();
    let decision_path = record_dir.join(format!("{}.decision.json", staged.revision));
    symlink(&outside_decision, &decision_path).unwrap();
    assert!(store.inspect_model_state().is_err());
    assert_eq!(fs::read(&outside_decision).unwrap(), b"decision");
    fs::remove_file(&decision_path).unwrap();
    fs::remove_file(outside_request).unwrap();
    fs::remove_file(outside_decision).unwrap();
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn mismatched_and_malformed_review_records_fail_state_inspection() {
    let (dir, store, revision) = fixture();
    let staged = stage(&store, revision);
    store
        .begin_candidate_review("raw-adc-domain-framing", &staged.revision)
        .unwrap();
    let request_path = dir.join(format!(
        ".rmwm/reviews/raw-adc-domain-framing/{}.request.json",
        staged.revision
    ));
    fs::write(
        &request_path,
        br#"{"artifact_id":"wrong","candidate_revision":"wrong"}"#,
    )
    .unwrap();
    assert!(store.inspect_model_state().is_err());
    fs::write(&request_path, b"not json").unwrap();
    assert!(store.inspect_model_state().is_err());
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn mismatched_decision_fails_state_inspection() {
    let (dir, store, revision) = fixture();
    let staged = stage(&store, revision);
    store
        .begin_candidate_review("raw-adc-domain-framing", &staged.revision)
        .unwrap();
    let decision_path = dir.join(format!(
        ".rmwm/reviews/raw-adc-domain-framing/{}.decision.json",
        staged.revision
    ));
    fs::write(&decision_path, format!(r#"{{"artifact_id":"wrong","candidate_revision":"{}","decision":"approved","decided_by":"reviewer"}}"#, staged.revision)).unwrap();
    assert!(store.inspect_model_state().is_err());
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn decision_without_request_fails_state_inspection() {
    let (dir, store, revision) = fixture();
    let staged = stage(&store, revision);
    store
        .begin_candidate_review("raw-adc-domain-framing", &staged.revision)
        .unwrap();
    let request_path = dir.join(format!(
        ".rmwm/reviews/raw-adc-domain-framing/{}.request.json",
        staged.revision
    ));
    fs::remove_file(request_path).unwrap();
    let decision_path = dir.join(format!(
        ".rmwm/reviews/raw-adc-domain-framing/{}.decision.json",
        staged.revision
    ));
    fs::write(
        decision_path,
        format!(r#"{{"artifact_id":"raw-adc-domain-framing","candidate_revision":"{}","decision":"approved","decided_by":"reviewer"}}"#, staged.revision),
    )
    .unwrap();
    assert!(store.inspect_model_state().is_err());
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn malformed_request_before_decision_fails_state_inspection() {
    let (dir, store, revision) = fixture();
    let staged = stage(&store, revision);
    store
        .begin_candidate_review("raw-adc-domain-framing", &staged.revision)
        .unwrap();
    let request_path = dir.join(format!(
        ".rmwm/reviews/raw-adc-domain-framing/{}.request.json",
        staged.revision
    ));
    fs::write(&request_path, b"not json").unwrap();
    let decision_path = dir.join(format!(
        ".rmwm/reviews/raw-adc-domain-framing/{}.decision.json",
        staged.revision
    ));
    fs::write(
        decision_path,
        format!(r#"{{"artifact_id":"raw-adc-domain-framing","candidate_revision":"{}","decision":"rejected","decided_by":"reviewer"}}"#, staged.revision),
    )
    .unwrap();
    assert!(store.inspect_model_state().is_err());
    fs::write(
        &request_path,
        br#"{"artifact_id":"wrong","candidate_revision":"wrong"}"#,
    )
    .unwrap();
    assert!(store.inspect_model_state().is_err());
    fs::remove_dir_all(dir).unwrap();
}

#[cfg(unix)]
#[test]
fn symlinked_review_directory_fails_state_inspection_instead_of_falling_back() {
    use std::os::unix::fs::symlink;
    let (dir, store, revision) = fixture();
    let staged = stage(&store, revision);
    let outside = dir.parent().unwrap().join("rmwm-review-directory-outside");
    fs::create_dir_all(&outside).unwrap();
    fs::create_dir_all(dir.join(".rmwm")).unwrap();
    symlink(&outside, dir.join(".rmwm/reviews")).unwrap();
    assert!(store.inspect_model_state().is_err());
    assert!(fs::read_dir(&outside).unwrap().next().is_none());
    assert_eq!(
        store
            .read_staged_candidate("raw-adc-domain-framing")
            .unwrap()
            .revision,
        staged.revision
    );
    fs::remove_file(dir.join(".rmwm/reviews")).unwrap();
    fs::remove_dir_all(&outside).unwrap();
    fs::remove_dir_all(dir).unwrap();
}
