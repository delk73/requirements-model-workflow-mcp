use requirements_model_workflow_mcp::{
    digest::{content_digest, revision_handle},
    model::CandidateIdentity,
    store::ModelStore,
};
use std::{
    collections::BTreeMap,
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

fn fixture() -> (PathBuf, ModelStore, Vec<u8>) {
    let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/raw-adc");
    let dir = std::env::temp_dir().join(format!(
        "rmwm-test-{}",
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
    let story = fs::read(dir.join("story.md")).unwrap();
    let store = ModelStore::open(&dir);
    (dir, store, story)
}

#[test]
fn raw_adc_story_reaches_staged_boundary_without_mutating_story() {
    let (dir, store, story) = fixture();
    let state = store.inspect_model_state().unwrap();
    assert_eq!(state.model_id, "raw-adc");
    assert_eq!(
        state
            .artifacts
            .iter()
            .find(|a| a.artifact_id == "raw-adc-story")
            .unwrap()
            .state,
        "accepted"
    );
    let accepted = store.read_accepted_artifact("raw-adc-story").unwrap();
    assert_eq!(accepted["bytes"].as_array().unwrap().len(), story.len());
    let story_revision = state
        .artifacts
        .iter()
        .find(|a| a.artifact_id == "raw-adc-story")
        .unwrap()
        .descriptor
        .accepted
        .as_ref()
        .unwrap()
        .revision
        .clone();
    let identity = CandidateIdentity {
        model_id: "raw-adc".into(),
        artifact_id: "raw-adc-domain-framing".into(),
        artifact_type: "domain_framing".into(),
        target_revision: None,
        source_revisions: BTreeMap::from([(String::from("raw-adc-story"), story_revision)]),
    };
    let candidate = store
        .stage_candidate(
            identity,
            "# Candidate framing\n\nMeaning remains agent-authored.",
        )
        .unwrap();
    assert_eq!(candidate.state, "staged");
    assert!(candidate.bytes.starts_with(b"---\nrmwm:\n"));
    assert_eq!(candidate.content.size, candidate.bytes.len());
    assert_eq!(fs::read(dir.join("story.md")).unwrap(), story);
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn stale_source_is_rejected() {
    let (_dir, store, _story) = fixture();
    let identity = CandidateIdentity {
        model_id: "raw-adc".into(),
        artifact_id: "raw-adc-domain-framing".into(),
        artifact_type: "domain_framing".into(),
        target_revision: None,
        source_revisions: BTreeMap::from([(
            String::from("raw-adc-story"),
            String::from("sha256:stale"),
        )]),
    };
    assert!(store
        .begin_candidate(identity)
        .unwrap_err()
        .contains("stale"));
}

#[test]
fn empty_source_bindings_are_rejected() {
    let (_dir, store, _story) = fixture();
    let identity = CandidateIdentity {
        model_id: "raw-adc".into(),
        artifact_id: "raw-adc-domain-framing".into(),
        artifact_type: "domain_framing".into(),
        target_revision: None,
        source_revisions: BTreeMap::new(),
    };
    assert!(store
        .begin_candidate(identity)
        .unwrap_err()
        .contains("exactly one source"));
}

#[test]
fn multiple_sources_are_rejected_for_domain_framing() {
    let (_dir, store, _story) = fixture();
    let identity = CandidateIdentity {
        model_id: "raw-adc".into(),
        artifact_id: "raw-adc-domain-framing".into(),
        artifact_type: "domain_framing".into(),
        target_revision: None,
        source_revisions: BTreeMap::from([
            ("raw-adc-story".into(), "sha256:story".into()),
            ("other-story".into(), "sha256:other".into()),
        ]),
    };
    assert!(store
        .begin_candidate(identity)
        .unwrap_err()
        .contains("exactly one source"));
}

#[test]
fn wrong_type_source_is_rejected_for_domain_framing() {
    let (_dir, store, _story) = fixture();
    let ontology_revision = store
        .inspect_model_state()
        .unwrap()
        .artifacts
        .iter()
        .find(|artifact| artifact.artifact_id == "raw-adc-domain-ontology")
        .unwrap()
        .descriptor
        .accepted
        .as_ref()
        .unwrap()
        .revision
        .clone();
    let identity = CandidateIdentity {
        model_id: "raw-adc".into(),
        artifact_id: "raw-adc-domain-framing".into(),
        artifact_type: "domain_framing".into(),
        target_revision: None,
        source_revisions: BTreeMap::from([("raw-adc-domain-ontology".into(), ontology_revision)]),
    };
    assert!(store
        .begin_candidate(identity)
        .unwrap_err()
        .contains("domain framing source must be a system story"));
}

#[test]
fn system_story_candidate_is_rejected() {
    let (_dir, store, _story) = fixture();
    let identity = CandidateIdentity {
        model_id: "raw-adc".into(),
        artifact_id: "raw-adc-story".into(),
        artifact_type: "system_story".into(),
        target_revision: None,
        source_revisions: BTreeMap::new(),
    };
    assert_eq!(
        store.begin_candidate(identity).unwrap_err(),
        "only domain framing candidates are supported"
    );
}

#[test]
fn domain_ontology_candidate_is_rejected() {
    let (_dir, store, _story) = fixture();
    let identity = CandidateIdentity {
        model_id: "raw-adc".into(),
        artifact_id: "raw-adc-domain-ontology".into(),
        artifact_type: "domain_ontology".into(),
        target_revision: None,
        source_revisions: BTreeMap::new(),
    };
    assert_eq!(
        store.begin_candidate(identity).unwrap_err(),
        "only domain framing candidates are supported"
    );
}

#[test]
fn revised_story_revision_can_bind_new_framing_candidate() {
    let (dir, store, story) = fixture();
    let revised = [story.as_slice(), b" revised"].concat();
    fs::write(dir.join("story.md"), &revised).unwrap();
    let revised_digest = requirements_model_workflow_mcp::digest::content_digest(&revised);
    let mut manifest = fs::read_to_string(dir.join("requirements_model.yaml")).unwrap();
    manifest = manifest.replace(
        "sha256:d9fc45a0fae8dccf8c4a6ddc7f13d1c4604775b0d1f03abfa92d8f4ec1ffe0ae",
        "sha256:updated-story-revision",
    );
    manifest = manifest.replace(
        "sha256:88f778b64cdca9bee76ec50fe7fb4d99a787617bf0d7cf6cf301ad2cac2477a4",
        &revised_digest,
    );
    manifest = manifest.replace("size: 532", &format!("size: {}", revised.len()));
    fs::write(dir.join("requirements_model.yaml"), manifest).unwrap();
    let identity = CandidateIdentity {
        model_id: "raw-adc".into(),
        artifact_id: "raw-adc-domain-framing".into(),
        artifact_type: "domain_framing".into(),
        target_revision: None,
        source_revisions: BTreeMap::from([(
            "raw-adc-story".into(),
            "sha256:updated-story-revision".into(),
        )]),
    };
    assert!(store.begin_candidate(identity).is_ok());
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn unsafe_artifact_id_cannot_escape_staged_directory() {
    let (_dir, store, _story) = fixture();
    let identity = CandidateIdentity {
        model_id: "raw-adc".into(),
        artifact_id: "../escape".into(),
        artifact_type: "domain_framing".into(),
        target_revision: None,
        source_revisions: BTreeMap::from([("raw-adc-story".into(), "sha256:story".into())]),
    };
    assert!(store
        .begin_candidate(identity)
        .unwrap_err()
        .contains("invalid artifact ID"));
}

#[cfg(unix)]
#[test]
fn symlinked_staging_directory_cannot_escape_model_root() {
    use std::os::unix::fs::symlink;
    let (dir, store, _story) = fixture();
    let outside = dir.parent().unwrap().join("rmwm-outside-staged");
    fs::create_dir_all(&outside).unwrap();
    fs::create_dir_all(dir.join(".rmwm")).unwrap();
    symlink(&outside, dir.join(".rmwm/staged")).unwrap();
    let identity = CandidateIdentity {
        model_id: "raw-adc".into(),
        artifact_id: "raw-adc-domain-framing".into(),
        artifact_type: "domain_framing".into(),
        target_revision: None,
        source_revisions: BTreeMap::from([(
            "raw-adc-story".into(),
            "sha256:d9fc45a0fae8dccf8c4a6ddc7f13d1c4604775b0d1f03abfa92d8f4ec1ffe0ae".into(),
        )]),
    };
    assert_eq!(
        store.stage_candidate(identity, "# Framing").unwrap_err(),
        "staged directory escapes model root"
    );
    assert!(fs::read_dir(&outside).unwrap().next().is_none());
    fs::remove_file(dir.join(".rmwm/staged")).unwrap();
    fs::remove_dir_all(&outside).unwrap();
    fs::remove_dir_all(dir).unwrap();
}

#[cfg(unix)]
#[test]
fn symlinked_rmwm_directory_cannot_escape_model_root() {
    use std::os::unix::fs::symlink;
    let (dir, store, _story) = fixture();
    let outside = dir.parent().unwrap().join("rmwm-outside-parent");
    fs::create_dir_all(&outside).unwrap();
    let before = fs::read_dir(&outside).unwrap().count();
    let story_revision = store
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
    symlink(&outside, dir.join(".rmwm")).unwrap();
    let identity = CandidateIdentity {
        model_id: "raw-adc".into(),
        artifact_id: "raw-adc-domain-framing".into(),
        artifact_type: "domain_framing".into(),
        target_revision: None,
        source_revisions: BTreeMap::from([("raw-adc-story".into(), story_revision)]),
    };
    assert_eq!(
        store.stage_candidate(identity, "# Framing").unwrap_err(),
        "rmwm directory escapes model root"
    );
    assert_eq!(fs::read_dir(&outside).unwrap().count(), before);
    assert!(fs::symlink_metadata(dir.join(".rmwm"))
        .unwrap()
        .file_type()
        .is_symlink());
    fs::remove_file(dir.join(".rmwm")).unwrap();
    fs::remove_dir_all(&outside).unwrap();
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn missing_staging_directories_are_created_inside_model_root() {
    let (dir, store, story) = fixture();
    let story_revision = store
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
    let identity = CandidateIdentity {
        model_id: "raw-adc".into(),
        artifact_id: "raw-adc-domain-framing".into(),
        artifact_type: "domain_framing".into(),
        target_revision: None,
        source_revisions: BTreeMap::from([("raw-adc-story".into(), story_revision)]),
    };
    store.stage_candidate(identity, "# Framing").unwrap();
    assert!(dir
        .join(".rmwm/staged/raw-adc-domain-framing.json")
        .exists());
    assert_eq!(fs::read(dir.join("story.md")).unwrap(), story);
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn existing_staged_candidate_is_not_overwritten() {
    let (dir, store, _story) = fixture();
    let story_revision = store
        .inspect_model_state()
        .unwrap()
        .artifacts
        .iter()
        .find(|a| a.artifact_id == "raw-adc-story")
        .unwrap()
        .descriptor
        .accepted
        .as_ref()
        .unwrap()
        .revision
        .clone();
    let identity = CandidateIdentity {
        model_id: "raw-adc".into(),
        artifact_id: "raw-adc-domain-framing".into(),
        artifact_type: "domain_framing".into(),
        target_revision: None,
        source_revisions: BTreeMap::from([(String::from("raw-adc-story"), story_revision)]),
    };
    store.stage_candidate(identity.clone(), "first").unwrap();
    assert!(store.stage_candidate(identity, "second").is_err());
    let staged = fs::read(dir.join(".rmwm/staged/raw-adc-domain-framing.json")).unwrap();
    let staged: requirements_model_workflow_mcp::model::StagedCandidate =
        serde_json::from_slice(&staged).unwrap();
    assert!(String::from_utf8_lossy(&staged.bytes).contains("first"));
    assert!(!String::from_utf8_lossy(&staged.bytes).contains("second"));
    fs::remove_dir_all(dir).unwrap();
}

#[cfg(unix)]
#[test]
fn symlink_artifact_path_cannot_escape_model_root() {
    use std::os::unix::fs::symlink;
    let (dir, store, _story) = fixture();
    let outside = dir.parent().unwrap().join("rmwm-outside-story.md");
    fs::write(&outside, b"outside").unwrap();
    fs::remove_file(dir.join("story.md")).unwrap();
    symlink(&outside, dir.join("story.md")).unwrap();
    assert!(store.read_accepted_artifact("raw-adc-story").is_err());
    fs::remove_file(dir.join("story.md")).unwrap();
    fs::remove_file(outside).unwrap();
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn modified_source_file_is_rejected_even_when_manifest_revision_is_unchanged() {
    let (dir, store, story) = fixture();
    fs::write(
        dir.join("story.md"),
        [story.as_slice(), b"changed"].concat(),
    )
    .unwrap();
    let identity = CandidateIdentity {
        model_id: "raw-adc".into(),
        artifact_id: "raw-adc-domain-framing".into(),
        artifact_type: "domain_framing".into(),
        target_revision: None,
        source_revisions: BTreeMap::from([(
            String::from("raw-adc-story"),
            String::from("sha256:d9fc45a0fae8dccf8c4a6ddc7f13d1c4604775b0d1f03abfa92d8f4ec1ffe0ae"),
        )]),
    };
    assert!(store
        .begin_candidate(identity)
        .unwrap_err()
        .contains("stale artifact"));
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn altered_or_missing_accepted_artifact_is_rejected() {
    let (dir, store, story) = fixture();
    fs::write(
        dir.join("story.md"),
        [story.as_slice(), b"changed"].concat(),
    )
    .unwrap();
    assert!(store.read_accepted_artifact("raw-adc-story").is_err());
    fs::remove_file(dir.join("story.md")).unwrap();
    assert!(store.read_accepted_artifact("raw-adc-story").is_err());
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn candidate_frontmatter_and_line_endings_are_controlled() {
    let (dir, store, _story) = fixture();
    let story_revision = store
        .inspect_model_state()
        .unwrap()
        .artifacts
        .iter()
        .find(|a| a.artifact_id == "raw-adc-story")
        .unwrap()
        .descriptor
        .accepted
        .as_ref()
        .unwrap()
        .revision
        .clone();
    let identity = CandidateIdentity {
        model_id: "raw-adc".into(),
        artifact_id: "raw-adc-domain-framing".into(),
        artifact_type: "domain_framing".into(),
        target_revision: None,
        source_revisions: BTreeMap::from([(String::from("raw-adc-story"), story_revision)]),
    };
    let candidate = store
        .stage_candidate(identity, "# Heading\r\n\r\nBody")
        .unwrap();
    assert!(!candidate.bytes.windows(2).any(|pair| pair == b"\r\n"));
    assert!(candidate.bytes.ends_with(b"\n"));
    assert!(String::from_utf8(candidate.bytes).unwrap().starts_with("---\nrmwm:\n  schema: \"artifact/v1\"\n  id: \"raw-adc-domain-framing\"\n  type: \"domain_framing\"\n---\n"));
    assert!(dir
        .join(".rmwm/staged/raw-adc-domain-framing.json")
        .exists());
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn digest_and_revision_handles_are_deterministic_and_source_ordered() {
    assert_eq!(
        content_digest(b"abc"),
        "sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    let first = revision_handle(
        "m",
        "a",
        "domain_framing",
        "domain_framing",
        "sha256:x",
        &[("z".into(), "2".into()), ("b".into(), "1".into())],
    );
    let second = revision_handle(
        "m",
        "a",
        "domain_framing",
        "domain_framing",
        "sha256:x",
        &[("b".into(), "1".into()), ("z".into(), "2".into())],
    );
    assert_eq!(first, second);
}
