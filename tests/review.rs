use fs2::FileExt;
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
        .find("\n  raw-adc-domain-ontology:")
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

fn accepted_target_fixture() -> (PathBuf, ModelStore, String, String) {
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/raw-adc");
    let dir = std::env::temp_dir().join(format!(
        "rmwm-review-accepted-{}",
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
    let story_revision = state
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
    let framing_revision = state
        .artifacts
        .iter()
        .find(|artifact| artifact.artifact_id == "raw-adc-domain-framing")
        .unwrap()
        .descriptor
        .accepted
        .as_ref()
        .unwrap()
        .revision
        .clone();
    (dir, store, story_revision, framing_revision)
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

fn ontology_identity(target_revision: String, framing_revision: String) -> CandidateIdentity {
    CandidateIdentity {
        model_id: "raw-adc".into(),
        artifact_id: "raw-adc-domain-ontology".into(),
        artifact_type: "domain_ontology".into(),
        target_revision: Some(target_revision),
        source_revisions: BTreeMap::from([("raw-adc-domain-framing".into(), framing_revision)]),
    }
}

const ONTOLOGY_BODY: &str = "# Ontology\n\n## Concepts\n\n| ID | Concept |\n| --- | --- |\n| `concept.c001` | Capture |\n\n## Properties\n\n| ID | Property |\n| --- | --- |\n| `property.p001` | Capture identity |\n\n## Relationships\n\n| ID | Relationship |\n| --- | --- |\n| `relationship.r001` | relates to |\n\n## Constraints\n\n* `constraint.k001` A constraint.\n";

fn with_ontology_ids(body: &str) -> String {
    let mut section = "";
    let mut table_line = 0;
    let mut element_number = 0;
    let mut result = String::new();

    for line in body.lines() {
        match line {
            "## Concepts" => {
                section = "concept.c";
                table_line = 0;
                element_number = 0;
            }
            "## Properties" => {
                section = "property.p";
                table_line = 0;
                element_number = 0;
            }
            "## Relationships" => {
                section = "relationship.r";
                table_line = 0;
                element_number = 0;
            }
            "## Constraints" => {
                section = "constraint.k";
                element_number = 0;
            }
            heading if heading.starts_with("## ") => section = "",
            _ => {}
        }

        let transformed =
            if !section.is_empty() && section != "constraint.k" && line.starts_with('|') {
                table_line += 1;
                match table_line {
                    1 => format!("| ID |{}", line.strip_prefix('|').unwrap()),
                    2 => format!("| --- |{}", line.strip_prefix('|').unwrap()),
                    _ => {
                        element_number += 1;
                        format!(
                            "| `{}{element_number:03}` |{}",
                            section,
                            line.strip_prefix('|').unwrap()
                        )
                    }
                }
            } else if section == "constraint.k" && line.starts_with("* ") {
                element_number += 1;
                format!(
                    "* `{}{element_number:03}` {}",
                    section,
                    line.strip_prefix("* ").unwrap()
                )
            } else {
                line.into()
            };
        result.push_str(&transformed);
        result.push('\n');
    }
    result
}

fn accepted_ontology_revision(store: &ModelStore) -> String {
    store
        .inspect_model_state()
        .unwrap()
        .artifacts
        .into_iter()
        .find(|artifact| artifact.artifact_id == "raw-adc-domain-ontology")
        .unwrap()
        .descriptor
        .accepted
        .unwrap()
        .revision
}

#[test]
fn ontology_candidate_requires_exactly_one_current_framing_source() {
    let (dir, store, _story_revision, framing_revision) = accepted_target_fixture();
    let ontology_revision = accepted_ontology_revision(&store);

    store
        .begin_candidate(ontology_identity(
            ontology_revision.clone(),
            framing_revision.clone(),
        ))
        .unwrap();

    let mut missing = ontology_identity(ontology_revision.clone(), framing_revision.clone());
    missing.source_revisions.clear();
    assert!(store
        .begin_candidate(missing)
        .unwrap_err()
        .contains("exactly one source"));

    let mut multiple = ontology_identity(ontology_revision.clone(), framing_revision.clone());
    multiple
        .source_revisions
        .insert("raw-adc-story".into(), "sha256:story".into());
    assert!(store
        .begin_candidate(multiple)
        .unwrap_err()
        .contains("exactly one source"));

    let mut wrong_type = ontology_identity(ontology_revision.clone(), framing_revision.clone());
    wrong_type.source_revisions.clear();
    wrong_type
        .source_revisions
        .insert("raw-adc-story".into(), "sha256:story".into());
    assert!(store
        .begin_candidate(wrong_type)
        .unwrap_err()
        .contains("domain ontology source must be a domain framing"));

    let stale_source = ontology_identity(ontology_revision, "sha256:stale".into());
    assert!(store
        .begin_candidate(stale_source)
        .unwrap_err()
        .contains("stale or incorrect source revision"));
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn ontology_candidate_rejects_stale_target_revision() {
    let (dir, store, _story_revision, framing_revision) = accepted_target_fixture();
    let identity = ontology_identity("sha256:stale".into(), framing_revision);
    assert!(store
        .begin_candidate(identity)
        .unwrap_err()
        .contains("stale or incorrect target revision"));
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn ontology_candidate_can_be_staged_read_reviewed_and_rejected() {
    let (dir, store, _story_revision, framing_revision) = accepted_target_fixture();
    let candidate = store
        .stage_candidate(
            ontology_identity(accepted_ontology_revision(&store), framing_revision),
            ONTOLOGY_BODY,
        )
        .unwrap();
    assert_eq!(
        store
            .read_staged_candidate("raw-adc-domain-ontology", None, None)
            .unwrap()
            .text
            .into_bytes(),
        candidate.bytes
    );
    store
        .begin_candidate_review("raw-adc-domain-ontology", &candidate.revision)
        .unwrap();
    store
        .record_candidate_decision(
            "raw-adc-domain-ontology",
            &candidate.revision,
            "rejected",
            "reviewer".into(),
            Some("needs revision".into()),
        )
        .unwrap();
    assert_eq!(
        store
            .inspect_model_state()
            .unwrap()
            .artifacts
            .into_iter()
            .find(|artifact| artifact.artifact_id == "raw-adc-domain-ontology")
            .unwrap()
            .state,
        "rejected"
    );
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn staged_ranged_reads_preserve_text_metadata_and_range_semantics() {
    let (dir, store, story_revision) = fixture();
    let candidate = stage(&store, story_revision);
    let unbounded = store
        .read_staged_candidate("raw-adc-domain-framing", None, None)
        .unwrap();
    assert_eq!(unbounded.text.as_bytes(), candidate.bytes.as_slice());
    assert_eq!(unbounded.revision, candidate.revision);
    assert_eq!(unbounded.identity, candidate.identity);
    assert_eq!(unbounded.state, "staged");
    assert!(unbounded.total_lines.is_none());

    let first = store
        .read_staged_candidate("raw-adc-domain-framing", Some(1), Some(2))
        .unwrap();
    let middle = store
        .read_staged_candidate("raw-adc-domain-framing", Some(3), Some(4))
        .unwrap();
    let through_eof = store
        .read_staged_candidate("raw-adc-domain-framing", Some(5), None)
        .unwrap();
    let mut reconstructed = first.text.clone();
    reconstructed.push_str(&middle.text);
    reconstructed.push_str(&through_eof.text);
    assert_eq!(reconstructed, unbounded.text);
    assert_eq!(first.revision, unbounded.revision);
    assert_eq!(first.identity, unbounded.identity);
    assert_eq!(first.state, unbounded.state);
    assert_eq!(through_eof.end_line, through_eof.total_lines);

    let clamped = store
        .read_staged_candidate("raw-adc-domain-framing", Some(1), Some(1000))
        .unwrap();
    assert_eq!(clamped.text, unbounded.text);
    assert_eq!(clamped.end_line, clamped.total_lines);
    for (start, end) in [(None, Some(1)), (Some(4), Some(2)), (Some(1000), None)] {
        assert!(store
            .read_staged_candidate("raw-adc-domain-framing", start, end)
            .is_err());
    }
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn large_staged_ontology_is_consumable_through_bounded_reads() {
    let (dir, store, _story_revision, framing_revision) = accepted_target_fixture();
    let ontology = fs::read_to_string(dir.join("domain_ontology.md")).unwrap();
    let body = with_ontology_ids(ontology.split_once("\n\n").unwrap().1);
    let candidate = store
        .stage_candidate(
            ontology_identity(accepted_ontology_revision(&store), framing_revision),
            &body,
        )
        .unwrap();
    let whole = store
        .read_staged_candidate("raw-adc-domain-ontology", None, None)
        .unwrap();
    let chunks = [
        store
            .read_staged_candidate("raw-adc-domain-ontology", Some(1), Some(40))
            .unwrap(),
        store
            .read_staged_candidate("raw-adc-domain-ontology", Some(41), Some(80))
            .unwrap(),
        store
            .read_staged_candidate("raw-adc-domain-ontology", Some(81), Some(120))
            .unwrap(),
        store
            .read_staged_candidate("raw-adc-domain-ontology", Some(121), Some(141))
            .unwrap(),
        store
            .read_staged_candidate("raw-adc-domain-ontology", Some(142), None)
            .unwrap(),
    ];
    let reconstructed = chunks
        .iter()
        .map(|chunk| chunk.text.as_str())
        .collect::<String>();
    assert_eq!(reconstructed, whole.text);
    assert_eq!(whole.text.as_bytes(), candidate.bytes.as_slice());
    assert!(chunks
        .iter()
        .all(|chunk| chunk.text.len() < whole.text.len() / 2));
    assert!(chunks
        .iter()
        .all(|chunk| chunk.revision == candidate.revision));
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn approved_ontology_candidate_updates_only_ontology_and_binds_new_framing() {
    let (dir, store, _story_revision, framing_revision) = accepted_target_fixture();
    let framing_before = fs::read(dir.join("domain_framing.md")).unwrap();
    let story_before = fs::read(dir.join("story.md")).unwrap();
    let candidate = store
        .stage_candidate(
            ontology_identity(accepted_ontology_revision(&store), framing_revision.clone()),
            ONTOLOGY_BODY,
        )
        .unwrap();
    store
        .begin_candidate_review("raw-adc-domain-ontology", &candidate.revision)
        .unwrap();
    store
        .record_candidate_decision(
            "raw-adc-domain-ontology",
            &candidate.revision,
            "approved",
            "reviewer".into(),
            None,
        )
        .unwrap();
    let accepted = store
        .accept_candidate("raw-adc-domain-ontology", &candidate.revision)
        .unwrap();

    assert_eq!(accepted.content, candidate.content);
    assert_eq!(
        fs::read(dir.join("domain_ontology.md")).unwrap(),
        candidate.bytes
    );
    assert_eq!(
        accepted.sources.get("raw-adc-domain-framing"),
        Some(&framing_revision)
    );
    assert_eq!(
        fs::read(dir.join("domain_framing.md")).unwrap(),
        framing_before
    );
    assert_eq!(fs::read(dir.join("story.md")).unwrap(), story_before);
    let ontology = store
        .inspect_model_state()
        .unwrap()
        .artifacts
        .into_iter()
        .find(|artifact| artifact.artifact_id == "raw-adc-domain-ontology")
        .unwrap();
    assert_eq!(ontology.state, "accepted");
    assert_eq!(
        ontology.descriptor.accepted.unwrap().revision,
        candidate.revision
    );
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn revised_framing_can_be_followed_by_ontology_reacceptance() {
    let (dir, store, story_revision, framing_a_revision) = accepted_target_fixture();
    let ontology_o_revision = accepted_ontology_revision(&store);
    let framing_before = fs::read(dir.join("domain_framing.md")).unwrap();
    let story_before = fs::read(dir.join("story.md")).unwrap();

    let framing_candidate = store
        .stage_candidate(
            CandidateIdentity {
                model_id: "raw-adc".into(),
                artifact_id: "raw-adc-domain-framing".into(),
                artifact_type: "domain_framing".into(),
                target_revision: Some(framing_a_revision.clone()),
                source_revisions: BTreeMap::from([(
                    "raw-adc-story".into(),
                    story_revision.clone(),
                )]),
            },
            "# Framing B",
        )
        .unwrap();
    store
        .begin_candidate_review("raw-adc-domain-framing", &framing_candidate.revision)
        .unwrap();
    store
        .record_candidate_decision(
            "raw-adc-domain-framing",
            &framing_candidate.revision,
            "approved",
            "reviewer".into(),
            None,
        )
        .unwrap();
    let framing_b = store
        .accept_candidate("raw-adc-domain-framing", &framing_candidate.revision)
        .unwrap();
    assert_eq!(framing_b.revision, framing_candidate.revision);

    let state = store.inspect_model_state().unwrap();
    let ontology_after_framing = state
        .artifacts
        .iter()
        .find(|artifact| artifact.artifact_id == "raw-adc-domain-ontology")
        .unwrap();
    assert_eq!(ontology_after_framing.state, "review_required");
    assert_eq!(
        ontology_after_framing
            .descriptor
            .accepted
            .as_ref()
            .unwrap()
            .revision,
        ontology_o_revision
    );
    assert_eq!(
        ontology_after_framing
            .descriptor
            .accepted
            .as_ref()
            .unwrap()
            .sources
            .get("raw-adc-domain-framing"),
        Some(&framing_a_revision)
    );

    let ontology_candidate = store
        .stage_candidate(
            ontology_identity(ontology_o_revision, framing_b.revision.clone()),
            ONTOLOGY_BODY,
        )
        .unwrap();
    store
        .begin_candidate_review("raw-adc-domain-ontology", &ontology_candidate.revision)
        .unwrap();
    store
        .record_candidate_decision(
            "raw-adc-domain-ontology",
            &ontology_candidate.revision,
            "approved",
            "reviewer".into(),
            None,
        )
        .unwrap();
    let ontology_accepted = store
        .accept_candidate("raw-adc-domain-ontology", &ontology_candidate.revision)
        .unwrap();

    assert_eq!(ontology_accepted.revision, ontology_candidate.revision);
    assert_eq!(
        ontology_accepted.sources.get("raw-adc-domain-framing"),
        Some(&framing_b.revision)
    );
    assert_eq!(
        fs::read(dir.join("domain_framing.md")).unwrap(),
        framing_candidate.bytes
    );
    assert_eq!(fs::read(dir.join("story.md")).unwrap(), story_before);
    assert_eq!(
        store
            .inspect_model_state()
            .unwrap()
            .artifacts
            .into_iter()
            .find(|artifact| artifact.artifact_id == "raw-adc-domain-framing")
            .unwrap()
            .descriptor
            .accepted
            .unwrap()
            .revision,
        framing_b.revision
    );
    assert_eq!(
        store
            .inspect_model_state()
            .unwrap()
            .artifacts
            .into_iter()
            .find(|artifact| artifact.artifact_id == "raw-adc-domain-ontology")
            .unwrap()
            .state,
        "accepted"
    );
    assert!(store
        .report_affected_downstream_artifacts("raw-adc-domain-framing")
        .unwrap()
        .affected_artifacts
        .is_empty());
    assert_eq!(
        fs::read(dir.join("domain_framing.md")).unwrap(),
        framing_candidate.bytes
    );
    assert_eq!(fs::read(dir.join("story.md")).unwrap(), story_before);
    assert_ne!(framing_before, framing_candidate.bytes);
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn exact_staged_candidate_can_be_read_and_reviewed_without_mutating_inputs() {
    let (dir, store, revision) = fixture();
    let manifest = fs::read(dir.join("requirements_model.yaml")).unwrap();
    let story = fs::read(dir.join("story.md")).unwrap();
    let staged = stage(&store, revision);
    assert_eq!(staged.supersedes, None);
    let staged_path = dir.join(".rmwm/staged/raw-adc-domain-framing.json");
    let staged_bytes = fs::read(&staged_path).unwrap();
    assert!(!String::from_utf8_lossy(&staged_bytes).contains("supersedes"));
    let staged_state = store
        .inspect_model_state()
        .unwrap()
        .artifacts
        .into_iter()
        .find(|artifact| artifact.artifact_id == "raw-adc-domain-framing")
        .unwrap();
    assert_eq!(staged_state.state, "staged");
    assert_eq!(
        staged_state.candidate_revision.as_deref(),
        Some(staged.revision.as_str())
    );
    assert_eq!(
        store
            .read_staged_candidate("raw-adc-domain-framing", None, None)
            .unwrap()
            .text
            .into_bytes(),
        staged.bytes
    );
    let request = store
        .begin_candidate_review("raw-adc-domain-framing", &staged.revision)
        .unwrap();
    assert_eq!(request.candidate_revision, staged.revision);
    let review_state = store
        .inspect_model_state()
        .unwrap()
        .artifacts
        .into_iter()
        .find(|artifact| artifact.artifact_id == "raw-adc-domain-framing")
        .unwrap();
    assert_eq!(review_state.state, "under_review");
    assert_eq!(
        review_state.candidate_revision.as_deref(),
        Some(staged.revision.as_str())
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
        .read_staged_candidate("raw-adc-domain-framing", None, None)
        .is_err());
    assert!(store
        .begin_candidate_review("raw-adc-domain-framing", &staged.revision)
        .is_err());
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn approval_rejection_and_duplicate_decisions_are_immutable() {
    let (dir, store, revision) = fixture();
    let manifest = fs::read(dir.join("requirements_model.yaml")).unwrap();
    let story = fs::read(dir.join("story.md")).unwrap();
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
    let approved_state = store
        .inspect_model_state()
        .unwrap()
        .artifacts
        .into_iter()
        .find(|artifact| artifact.artifact_id == "raw-adc-domain-framing")
        .unwrap();
    assert_eq!(approved_state.state, "approved");
    assert_eq!(
        approved_state.candidate_revision.as_deref(),
        Some(staged.revision.as_str())
    );
    assert_eq!(
        fs::read(dir.join("requirements_model.yaml")).unwrap(),
        manifest
    );
    assert_eq!(fs::read(dir.join("story.md")).unwrap(), story);
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
fn approval_of_revision_preserves_accepted_target_and_manifest() {
    let (dir, store, story_revision, accepted_revision) = accepted_target_fixture();
    let manifest = fs::read(dir.join("requirements_model.yaml")).unwrap();
    let accepted_framing = fs::read(dir.join("domain_framing.md")).unwrap();
    let staged = store
        .stage_candidate(
            CandidateIdentity {
                model_id: "raw-adc".into(),
                artifact_id: "raw-adc-domain-framing".into(),
                artifact_type: "domain_framing".into(),
                target_revision: Some(accepted_revision.clone()),
                source_revisions: BTreeMap::from([("raw-adc-story".into(), story_revision)]),
            },
            "# Revised framing",
        )
        .unwrap();
    store
        .begin_candidate_review("raw-adc-domain-framing", &staged.revision)
        .unwrap();
    store
        .record_candidate_decision(
            "raw-adc-domain-framing",
            &staged.revision,
            "approved",
            "reviewer".into(),
            None,
        )
        .unwrap();

    let approved = store
        .inspect_model_state()
        .unwrap()
        .artifacts
        .into_iter()
        .find(|artifact| artifact.artifact_id == "raw-adc-domain-framing")
        .unwrap();
    assert_eq!(approved.state, "approved");
    assert_eq!(
        approved.descriptor.accepted.unwrap().revision,
        accepted_revision
    );
    assert_eq!(approved.candidate_revision, Some(staged.revision));
    assert_eq!(
        fs::read(dir.join("requirements_model.yaml")).unwrap(),
        manifest
    );
    assert_eq!(
        fs::read(dir.join("domain_framing.md")).unwrap(),
        accepted_framing
    );
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn approved_candidate_is_accepted_without_changing_ontology() {
    let (dir, store, story_revision, prior_framing_revision) = accepted_target_fixture();
    let ontology_before = fs::read(dir.join("domain_ontology.md")).unwrap();
    let candidate = store
        .stage_candidate(
            CandidateIdentity {
                model_id: "raw-adc".into(),
                artifact_id: "raw-adc-domain-framing".into(),
                artifact_type: "domain_framing".into(),
                target_revision: Some(prior_framing_revision.clone()),
                source_revisions: BTreeMap::from([(
                    "raw-adc-story".into(),
                    story_revision.clone(),
                )]),
            },
            "# Accepted revised framing",
        )
        .unwrap();
    store
        .begin_candidate_review("raw-adc-domain-framing", &candidate.revision)
        .unwrap();
    store
        .record_candidate_decision(
            "raw-adc-domain-framing",
            &candidate.revision,
            "approved",
            "reviewer".into(),
            None,
        )
        .unwrap();

    let accepted = store
        .accept_candidate("raw-adc-domain-framing", &candidate.revision)
        .unwrap();

    assert_eq!(accepted.revision, candidate.revision);
    assert_eq!(accepted.content, candidate.content);
    assert_eq!(accepted.sources.get("raw-adc-story"), Some(&story_revision));
    assert_eq!(
        fs::read(dir.join("domain_framing.md")).unwrap(),
        candidate.bytes
    );
    assert!(!dir
        .join(".rmwm/staged/raw-adc-domain-framing.json")
        .exists());
    assert_eq!(
        fs::read(dir.join("domain_ontology.md")).unwrap(),
        ontology_before
    );
    let state = store.inspect_model_state().unwrap();
    let framing = state
        .artifacts
        .iter()
        .find(|artifact| artifact.artifact_id == "raw-adc-domain-framing")
        .unwrap();
    assert_eq!(framing.state, "accepted");
    assert_eq!(
        framing.descriptor.accepted.as_ref().unwrap().revision,
        candidate.revision
    );
    let ontology = state
        .artifacts
        .iter()
        .find(|artifact| artifact.artifact_id == "raw-adc-domain-ontology")
        .unwrap();
    assert_eq!(
        ontology
            .descriptor
            .accepted
            .as_ref()
            .unwrap()
            .sources
            .get("raw-adc-domain-framing"),
        Some(&prior_framing_revision)
    );
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn acceptance_is_refused_while_the_requirements_model_is_locked() {
    let (dir, store, story_revision) = fixture();
    let candidate = stage(&store, story_revision);
    store
        .begin_candidate_review("raw-adc-domain-framing", &candidate.revision)
        .unwrap();
    store
        .record_candidate_decision(
            "raw-adc-domain-framing",
            &candidate.revision,
            "approved",
            "reviewer".into(),
            None,
        )
        .unwrap();
    let manifest_before = fs::read(dir.join("requirements_model.yaml")).unwrap();
    let lock_file = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .open(dir.join(".rmwm/requirements-model.accept.lock"))
        .unwrap();
    lock_file.try_lock_exclusive().unwrap();

    assert_eq!(
        store
            .accept_candidate("raw-adc-domain-framing", &candidate.revision)
            .unwrap_err(),
        "requirements model is locked"
    );
    assert_eq!(
        fs::read(dir.join("requirements_model.yaml")).unwrap(),
        manifest_before
    );
    assert!(dir
        .join(".rmwm/staged/raw-adc-domain-framing.json")
        .exists());
    lock_file.unlock().unwrap();
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn manifest_replacement_reports_acceptance_after_staged_cleanup() {
    let (dir, store, story_revision) = fixture();
    let candidate = stage(&store, story_revision);
    store
        .begin_candidate_review("raw-adc-domain-framing", &candidate.revision)
        .unwrap();
    store
        .record_candidate_decision(
            "raw-adc-domain-framing",
            &candidate.revision,
            "approved",
            "reviewer".into(),
            None,
        )
        .unwrap();
    let accepted = store
        .accept_candidate("raw-adc-domain-framing", &candidate.revision)
        .unwrap();

    assert_eq!(accepted.revision, candidate.revision);
    assert_eq!(
        fs::read(dir.join("domain_framing.md")).unwrap(),
        candidate.bytes
    );
    assert!(!dir
        .join(".rmwm/staged/raw-adc-domain-framing.json")
        .exists());
    assert!(!dir
        .join(".rmwm/recovery/raw-adc-domain-framing.accept.json")
        .exists());
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn acceptance_requires_the_exact_approved_current_candidate() {
    let (dir, store, story_revision) = fixture();
    let candidate = stage(&store, story_revision);
    assert_eq!(
        store
            .accept_candidate("raw-adc-domain-framing", &candidate.revision)
            .unwrap_err(),
        "candidate is not approved"
    );
    store
        .begin_candidate_review("raw-adc-domain-framing", &candidate.revision)
        .unwrap();
    store
        .record_candidate_decision(
            "raw-adc-domain-framing",
            &candidate.revision,
            "approved",
            "reviewer".into(),
            None,
        )
        .unwrap();
    assert!(store
        .accept_candidate("raw-adc-domain-framing", "sha256:wrong")
        .is_err());
    assert!(dir
        .join(".rmwm/staged/raw-adc-domain-framing.json")
        .exists());
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn stale_accepted_artifact_prevents_acceptance() {
    let (dir, store, story_revision, accepted_revision) = accepted_target_fixture();
    let candidate = store
        .stage_candidate(
            CandidateIdentity {
                model_id: "raw-adc".into(),
                artifact_id: "raw-adc-domain-framing".into(),
                artifact_type: "domain_framing".into(),
                target_revision: Some(accepted_revision),
                source_revisions: BTreeMap::from([("raw-adc-story".into(), story_revision)]),
            },
            "# Candidate",
        )
        .unwrap();
    store
        .begin_candidate_review("raw-adc-domain-framing", &candidate.revision)
        .unwrap();
    store
        .record_candidate_decision(
            "raw-adc-domain-framing",
            &candidate.revision,
            "approved",
            "reviewer".into(),
            None,
        )
        .unwrap();
    fs::write(dir.join("domain_framing.md"), b"modified").unwrap();

    assert!(store
        .accept_candidate("raw-adc-domain-framing", &candidate.revision)
        .unwrap_err()
        .contains("stale artifact"));
    assert!(dir
        .join(".rmwm/staged/raw-adc-domain-framing.json")
        .exists());
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn current_accepted_source_binding_reports_accepted() {
    let (dir, store, _story_revision, _framing_revision) = accepted_target_fixture();
    let framing = store
        .inspect_model_state()
        .unwrap()
        .artifacts
        .into_iter()
        .find(|artifact| artifact.artifact_id == "raw-adc-domain-framing")
        .unwrap();
    assert_eq!(framing.state, "accepted");
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn current_direct_dependent_is_not_reported() {
    let (dir, store, story_revision, _framing_revision) = accepted_target_fixture();

    let report = store
        .report_affected_downstream_artifacts("raw-adc-story")
        .unwrap();
    assert_eq!(report.artifact_id, "raw-adc-story");
    assert_eq!(report.accepted_revision, story_revision);
    assert!(report.affected_artifacts.is_empty());
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn superseded_accepted_source_binding_reports_review_required() {
    let (dir, store, _story_revision, _framing_revision) = accepted_target_fixture();
    let manifest_path = dir.join("requirements_model.yaml");
    let mut manifest: serde_yaml::Value =
        serde_yaml::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
    manifest["artifacts"]["raw-adc-story"]["accepted"]["revision"] =
        serde_yaml::Value::String("sha256:superseded".into());
    fs::write(&manifest_path, serde_yaml::to_string(&manifest).unwrap()).unwrap();

    let framing = store
        .inspect_model_state()
        .unwrap()
        .artifacts
        .into_iter()
        .find(|artifact| artifact.artifact_id == "raw-adc-domain-framing")
        .unwrap();
    assert_eq!(framing.state, "review_required");
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn stale_direct_dependent_is_reported_with_both_revisions() {
    let (dir, store, story_revision, prior_framing_revision) = accepted_target_fixture();
    let candidate = store
        .stage_candidate(
            CandidateIdentity {
                model_id: "raw-adc".into(),
                artifact_id: "raw-adc-domain-framing".into(),
                artifact_type: "domain_framing".into(),
                target_revision: Some(prior_framing_revision.clone()),
                source_revisions: BTreeMap::from([("raw-adc-story".into(), story_revision)]),
            },
            "# Revised framing",
        )
        .unwrap();
    store
        .begin_candidate_review("raw-adc-domain-framing", &candidate.revision)
        .unwrap();
    store
        .record_candidate_decision(
            "raw-adc-domain-framing",
            &candidate.revision,
            "approved",
            "reviewer".into(),
            None,
        )
        .unwrap();
    let accepted = store
        .accept_candidate("raw-adc-domain-framing", &candidate.revision)
        .unwrap();

    let report = store
        .report_affected_downstream_artifacts("raw-adc-domain-framing")
        .unwrap();
    assert_eq!(report.accepted_revision, accepted.revision);
    let ontology = report
        .affected_artifacts
        .iter()
        .find(|artifact| artifact.artifact_id == "raw-adc-domain-ontology")
        .unwrap();
    assert_eq!(ontology.bound_source_revision, prior_framing_revision);
    assert_eq!(ontology.current_source_revision, accepted.revision);
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn unrelated_accepted_artifact_is_not_reported() {
    let (dir, store, _story_revision, framing_revision) = accepted_target_fixture();

    let report = store
        .report_affected_downstream_artifacts("raw-adc-domain-framing")
        .unwrap();
    assert_eq!(report.accepted_revision, framing_revision);
    assert!(report.affected_artifacts.is_empty());
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn report_requires_an_accepted_artifact() {
    let (dir, store, _story_revision) = fixture();
    assert_eq!(
        store
            .report_affected_downstream_artifacts("raw-adc-domain-framing")
            .unwrap_err(),
        "artifact has no accepted revision"
    );
    assert_eq!(
        store
            .report_affected_downstream_artifacts("missing")
            .unwrap_err(),
        "unknown artifact"
    );
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn stale_source_does_not_mask_later_malformed_source_binding() {
    let (dir, store, _story_revision, _framing_revision) = accepted_target_fixture();
    let manifest_path = dir.join("requirements_model.yaml");
    let mut manifest: serde_yaml::Value =
        serde_yaml::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
    manifest["artifacts"]["raw-adc-story"]["accepted"]["revision"] =
        serde_yaml::Value::String("sha256:stale".into());
    manifest["artifacts"]["raw-adc-domain-framing"]["accepted"]["sources"]["zz-missing"] =
        serde_yaml::Value::String("sha256:missing".into());
    fs::write(&manifest_path, serde_yaml::to_string(&manifest).unwrap()).unwrap();

    assert_eq!(
        store.inspect_model_state().unwrap_err(),
        "accepted source artifact does not exist: zz-missing"
    );
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn missing_accepted_source_binding_fails_inspection() {
    let (dir, store, _story_revision, _framing_revision) = accepted_target_fixture();
    let manifest_path = dir.join("requirements_model.yaml");
    let mut manifest: serde_yaml::Value =
        serde_yaml::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
    manifest["artifacts"]["raw-adc-domain-framing"]["accepted"]["sources"]["missing"] =
        serde_yaml::Value::String("sha256:missing".into());
    fs::write(&manifest_path, serde_yaml::to_string(&manifest).unwrap()).unwrap();

    assert!(store
        .inspect_model_state()
        .unwrap_err()
        .contains("does not exist"));
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn unaccepted_bound_source_fails_inspection() {
    let (dir, store, _story_revision, _framing_revision) = accepted_target_fixture();
    let manifest_path = dir.join("requirements_model.yaml");
    let mut manifest: serde_yaml::Value =
        serde_yaml::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
    manifest["artifacts"]["raw-adc-story"]["accepted"] = serde_yaml::Value::Null;
    fs::write(&manifest_path, serde_yaml::to_string(&manifest).unwrap()).unwrap();

    assert!(store
        .inspect_model_state()
        .unwrap_err()
        .contains("has no accepted revision"));
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn candidate_with_superseded_bound_source_is_stale_during_inspection() {
    let (dir, store, story_revision, framing_revision) = accepted_target_fixture();
    let candidate = store
        .stage_candidate(
            CandidateIdentity {
                model_id: "raw-adc".into(),
                artifact_id: "raw-adc-domain-framing".into(),
                artifact_type: "domain_framing".into(),
                target_revision: Some(framing_revision),
                source_revisions: BTreeMap::from([("raw-adc-story".into(), story_revision)]),
            },
            "# Candidate",
        )
        .unwrap();
    let manifest_path = dir.join("requirements_model.yaml");
    let mut manifest: serde_yaml::Value =
        serde_yaml::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
    manifest["artifacts"]["raw-adc-story"]["accepted"]["revision"] =
        serde_yaml::Value::String("sha256:superseded".into());
    fs::write(&manifest_path, serde_yaml::to_string(&manifest).unwrap()).unwrap();

    assert!(store.inspect_model_state().is_err());
    assert_eq!(
        store
            .read_staged_candidate("raw-adc-domain-framing", None, None)
            .unwrap_err(),
        "stale or incorrect source revision for raw-adc-story"
    );
    assert!(dir
        .join(".rmwm/staged/raw-adc-domain-framing.json")
        .exists());
    assert_ne!(candidate.revision, "sha256:superseded");
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
    let rejected_state = store
        .inspect_model_state()
        .unwrap()
        .artifacts
        .into_iter()
        .find(|artifact| artifact.artifact_id == "raw-adc-domain-framing")
        .unwrap();
    assert_eq!(rejected_state.state, "rejected");
    assert_eq!(
        rejected_state.candidate_revision.as_deref(),
        Some(staged.revision.as_str())
    );
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn rejected_candidate_is_replaced_without_losing_review_history() {
    let (dir, store, story_revision, accepted_revision) = accepted_target_fixture();
    let candidate_a = store
        .stage_candidate(
            CandidateIdentity {
                model_id: "raw-adc".into(),
                artifact_id: "raw-adc-domain-framing".into(),
                artifact_type: "domain_framing".into(),
                target_revision: Some(accepted_revision.clone()),
                source_revisions: BTreeMap::from([(
                    "raw-adc-story".into(),
                    story_revision.clone(),
                )]),
            },
            "# Rejected framing",
        )
        .unwrap();
    store
        .begin_candidate_review("raw-adc-domain-framing", &candidate_a.revision)
        .unwrap();
    store
        .record_candidate_decision(
            "raw-adc-domain-framing",
            &candidate_a.revision,
            "rejected",
            "reviewer".into(),
            Some("needs revision".into()),
        )
        .unwrap();
    let review_dir = dir.join(".rmwm/reviews/raw-adc-domain-framing");
    let request_path = review_dir.join(format!("{}.request.json", candidate_a.revision));
    let decision_path = review_dir.join(format!("{}.decision.json", candidate_a.revision));
    let request_before = fs::read(&request_path).unwrap();
    let decision_before = fs::read(&decision_path).unwrap();

    let candidate_b = store
        .stage_candidate(
            CandidateIdentity {
                model_id: "raw-adc".into(),
                artifact_id: "raw-adc-domain-framing".into(),
                artifact_type: "domain_framing".into(),
                target_revision: Some(accepted_revision.clone()),
                source_revisions: BTreeMap::from([(
                    "raw-adc-story".into(),
                    story_revision.clone(),
                )]),
            },
            "# Revised framing",
        )
        .unwrap();

    assert_ne!(candidate_b.content.digest, candidate_a.content.digest);
    assert_ne!(candidate_b.revision, candidate_a.revision);
    assert_eq!(
        candidate_b.supersedes.as_deref(),
        Some(candidate_a.revision.as_str())
    );
    assert_eq!(candidate_b.state, "staged");
    assert_eq!(
        candidate_b.identity.target_revision,
        Some(accepted_revision.clone())
    );
    assert_eq!(
        candidate_b.identity.source_revisions.get("raw-adc-story"),
        Some(&story_revision)
    );
    assert_eq!(fs::read(&request_path).unwrap(), request_before);
    assert_eq!(fs::read(&decision_path).unwrap(), decision_before);
    assert!(!review_dir
        .join(format!("{}.request.json", candidate_b.revision))
        .exists());
    assert_eq!(
        store
            .inspect_model_state()
            .unwrap()
            .artifacts
            .into_iter()
            .find(|artifact| artifact.artifact_id == "raw-adc-domain-framing")
            .unwrap()
            .state,
        "staged"
    );
    assert_eq!(
        store
            .inspect_model_state()
            .unwrap()
            .artifacts
            .into_iter()
            .find(|artifact| artifact.artifact_id == "raw-adc-domain-framing")
            .unwrap()
            .descriptor
            .accepted
            .unwrap()
            .revision,
        accepted_revision
    );
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn replacement_preparation_failure_preserves_rejected_active_candidate() {
    let (dir, store, story_revision) = fixture();
    let candidate = stage(&store, story_revision.clone());
    store
        .begin_candidate_review("raw-adc-domain-framing", &candidate.revision)
        .unwrap();
    store
        .record_candidate_decision(
            "raw-adc-domain-framing",
            &candidate.revision,
            "rejected",
            "reviewer".into(),
            None,
        )
        .unwrap();
    let staged_path = dir.join(".rmwm/staged/raw-adc-domain-framing.json");
    let active_before = fs::read(&staged_path).unwrap();
    fs::write(staged_path.with_extension("json.tmp"), b"occupied").unwrap();

    assert!(store
        .stage_candidate(identity(story_revision), "# Replacement")
        .is_err());
    assert_eq!(fs::read(&staged_path).unwrap(), active_before);
    assert_eq!(
        fs::read(staged_path.with_extension("json.tmp")).unwrap(),
        b"occupied"
    );
    assert_eq!(
        store
            .read_staged_candidate("raw-adc-domain-framing", None, None)
            .unwrap()
            .revision,
        candidate.revision
    );
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn identical_replacement_of_rejected_candidate_is_refused() {
    let (dir, store, story_revision) = fixture();
    let candidate = stage(&store, story_revision.clone());
    store
        .begin_candidate_review("raw-adc-domain-framing", &candidate.revision)
        .unwrap();
    store
        .record_candidate_decision(
            "raw-adc-domain-framing",
            &candidate.revision,
            "rejected",
            "reviewer".into(),
            None,
        )
        .unwrap();

    assert_eq!(
        store
            .stage_candidate(identity(story_revision), "# Framing")
            .unwrap_err(),
        "replacement candidate must have a new revision"
    );
    assert_eq!(
        store
            .read_staged_candidate("raw-adc-domain-framing", None, None)
            .unwrap()
            .revision,
        candidate.revision
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
        .read_staged_candidate("raw-adc-domain-framing", None, None)
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
            .read_staged_candidate("raw-adc-domain-framing", None, None)
            .unwrap()
            .revision,
        staged.revision
    );
    fs::remove_file(dir.join(".rmwm/reviews")).unwrap();
    fs::remove_dir_all(&outside).unwrap();
    fs::remove_dir_all(dir).unwrap();
}
