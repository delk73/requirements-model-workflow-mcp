use requirements_model_workflow_mcp::{model::CandidateIdentity, store::ModelStore};
use std::{
    collections::BTreeMap,
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

const ARTIFACT: &str = "raw-adc-domain-framing";
const STORY: &str = "raw-adc-story";
const BODY: &str = "# Framing\n";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SemanticState {
    Absent,
    Staged,
    UnderReview,
    Approved,
    Rejected,
    Accepted,
}
#[derive(Clone, Copy, Debug)]
enum Operation {
    BeginReview,
    Approve,
    Reject,
    Accept,
    Replace,
}
#[derive(Clone, Copy, Debug)]
enum ExpectedOutcome {
    Rejected,
    State(SemanticState),
}
struct TransitionCase {
    name: &'static str,
    start_state: SemanticState,
    operation: Operation,
    expected: ExpectedOutcome,
}

struct RevisionMismatchCase {
    name: &'static str,
    start_state: SemanticState,
    operation: Operation,
}

const WRONG_REVISION_TRANSITIONS: &[RevisionMismatchCase] = &[
    RevisionMismatchCase {
        name: "under_review -> begin review with wrong revision",
        start_state: SemanticState::UnderReview,
        operation: Operation::BeginReview,
    },
    RevisionMismatchCase {
        name: "under_review -> approve with wrong revision",
        start_state: SemanticState::UnderReview,
        operation: Operation::Approve,
    },
    RevisionMismatchCase {
        name: "approved -> accept with wrong revision",
        start_state: SemanticState::Approved,
        operation: Operation::Accept,
    },
];

const TRANSITIONS: &[TransitionCase] = &[
    TransitionCase {
        name: "absent -> review",
        start_state: SemanticState::Absent,
        operation: Operation::BeginReview,
        expected: ExpectedOutcome::Rejected,
    },
    TransitionCase {
        name: "absent -> decision",
        start_state: SemanticState::Absent,
        operation: Operation::Approve,
        expected: ExpectedOutcome::Rejected,
    },
    TransitionCase {
        name: "absent -> accept",
        start_state: SemanticState::Absent,
        operation: Operation::Accept,
        expected: ExpectedOutcome::Rejected,
    },
    TransitionCase {
        name: "staged -> review",
        start_state: SemanticState::Staged,
        operation: Operation::BeginReview,
        expected: ExpectedOutcome::State(SemanticState::UnderReview),
    },
    TransitionCase {
        name: "staged -> accept",
        start_state: SemanticState::Staged,
        operation: Operation::Accept,
        expected: ExpectedOutcome::Rejected,
    },
    TransitionCase {
        name: "staged -> replace",
        start_state: SemanticState::Staged,
        operation: Operation::Replace,
        expected: ExpectedOutcome::Rejected,
    },
    TransitionCase {
        name: "under_review -> approve",
        start_state: SemanticState::UnderReview,
        operation: Operation::Approve,
        expected: ExpectedOutcome::State(SemanticState::Approved),
    },
    TransitionCase {
        name: "under_review -> reject",
        start_state: SemanticState::UnderReview,
        operation: Operation::Reject,
        expected: ExpectedOutcome::State(SemanticState::Rejected),
    },
    TransitionCase {
        name: "under_review -> accept",
        start_state: SemanticState::UnderReview,
        operation: Operation::Accept,
        expected: ExpectedOutcome::Rejected,
    },
    TransitionCase {
        name: "approved -> accept",
        start_state: SemanticState::Approved,
        operation: Operation::Accept,
        expected: ExpectedOutcome::State(SemanticState::Accepted),
    },
    TransitionCase {
        name: "approved -> duplicate approval",
        start_state: SemanticState::Approved,
        operation: Operation::Approve,
        expected: ExpectedOutcome::Rejected,
    },
    TransitionCase {
        name: "approved -> conflicting rejection",
        start_state: SemanticState::Approved,
        operation: Operation::Reject,
        expected: ExpectedOutcome::Rejected,
    },
    TransitionCase {
        name: "rejected -> accept",
        start_state: SemanticState::Rejected,
        operation: Operation::Accept,
        expected: ExpectedOutcome::Rejected,
    },
    TransitionCase {
        name: "rejected -> replace",
        start_state: SemanticState::Rejected,
        operation: Operation::Replace,
        expected: ExpectedOutcome::State(SemanticState::Staged),
    },
    TransitionCase {
        name: "accepted -> stale replace",
        start_state: SemanticState::Accepted,
        operation: Operation::Replace,
        expected: ExpectedOutcome::Rejected,
    },
];

struct Fixture {
    dir: PathBuf,
    store: ModelStore,
    story_revision: String,
}
impl Fixture {
    fn new() -> Self {
        let source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/raw-adc");
        let dir = std::env::temp_dir().join(format!(
            "rmwm-review-state-machine-{}",
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
        let story_revision = accepted_revision(&store, STORY);
        Self {
            dir,
            store,
            story_revision,
        }
    }
    fn identity(&self, target_revision: Option<String>) -> CandidateIdentity {
        CandidateIdentity {
            model_id: "raw-adc".into(),
            artifact_id: ARTIFACT.into(),
            artifact_type: "domain_framing".into(),
            target_revision,
            source_revisions: BTreeMap::from([(STORY.into(), self.story_revision.clone())]),
        }
    }
    fn state(&self) -> SemanticState {
        match self
            .store
            .inspect_model_state()
            .unwrap()
            .artifacts
            .into_iter()
            .find(|a| a.artifact_id == ARTIFACT)
            .unwrap()
            .state
            .as_str()
        {
            "absent" => SemanticState::Absent,
            "staged" => SemanticState::Staged,
            "under_review" => SemanticState::UnderReview,
            "approved" => SemanticState::Approved,
            "rejected" => SemanticState::Rejected,
            "accepted" => SemanticState::Accepted,
            state => panic!("unexpected state: {state}"),
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

fn accepted_revision(store: &ModelStore, artifact_id: &str) -> String {
    store
        .inspect_model_state()
        .unwrap()
        .artifacts
        .into_iter()
        .find(|a| a.artifact_id == artifact_id)
        .unwrap()
        .descriptor
        .accepted
        .unwrap()
        .revision
}

struct InvariantSnapshot {
    manifest: Vec<u8>,
    accepted_bytes: Option<Vec<u8>>,
    accepted_revision: Option<String>,
    sources: BTreeMap<String, String>,
    state: SemanticState,
}
fn snapshot(fixture: &Fixture) -> InvariantSnapshot {
    let artifact = fixture
        .store
        .inspect_model_state()
        .unwrap()
        .artifacts
        .into_iter()
        .find(|a| a.artifact_id == ARTIFACT)
        .unwrap();
    let accepted = artifact.descriptor.accepted;
    InvariantSnapshot {
        manifest: fs::read(fixture.dir.join("requirements_model.yaml")).unwrap(),
        accepted_bytes: accepted
            .as_ref()
            .map(|_| fs::read(fixture.dir.join("domain_framing.md")).unwrap()),
        accepted_revision: accepted.as_ref().map(|a| a.revision.clone()),
        sources: accepted.map(|a| a.sources).unwrap_or_default(),
        state: fixture.state(),
    }
}
fn assert_rejected(
    fixture: &Fixture,
    name: &str,
    result: Result<impl std::fmt::Debug, String>,
    before: InvariantSnapshot,
) {
    assert!(result.is_err(), "{name}: operation unexpectedly succeeded");
    let after = snapshot(fixture);
    assert_eq!(after.manifest, before.manifest, "{name}: manifest changed");
    assert_eq!(
        after.accepted_bytes, before.accepted_bytes,
        "{name}: accepted bytes changed"
    );
    assert_eq!(
        after.accepted_revision, before.accepted_revision,
        "{name}: accepted revision changed"
    );
    assert_eq!(
        after.sources, before.sources,
        "{name}: accepted sources changed"
    );
    assert_eq!(after.state, before.state, "{name}: state changed");
}

fn decision_bytes(fixture: &Fixture, revision: &str) -> Vec<u8> {
    fs::read(
        fixture
            .dir
            .join(".rmwm")
            .join("reviews")
            .join(ARTIFACT)
            .join(format!("{revision}.decision.json")),
    )
    .unwrap()
}

fn setup(fixture: &Fixture, state: SemanticState) -> (Option<String>, Option<String>) {
    if state == SemanticState::Absent {
        return (None, None);
    }
    let staged = fixture
        .store
        .stage_candidate(fixture.identity(None), BODY)
        .unwrap();
    if state == SemanticState::Staged {
        return (Some(staged.revision), None);
    }
    fixture
        .store
        .begin_candidate_review(ARTIFACT, &staged.revision)
        .unwrap();
    if state == SemanticState::UnderReview {
        return (Some(staged.revision), None);
    }
    let decision = if state == SemanticState::Rejected {
        "rejected"
    } else {
        "approved"
    };
    fixture
        .store
        .record_candidate_decision(
            ARTIFACT,
            &staged.revision,
            decision,
            "reviewer".into(),
            None,
        )
        .unwrap();
    if state == SemanticState::Approved || state == SemanticState::Rejected {
        return (Some(staged.revision), None);
    }
    let accepted = fixture
        .store
        .accept_candidate(ARTIFACT, &staged.revision)
        .unwrap();
    (None, Some(accepted.revision))
}

#[test]
fn explicit_review_gate_transition_matrix() {
    for case in TRANSITIONS {
        let fixture = Fixture::new();
        let (revision, accepted) = setup(&fixture, case.start_state);
        assert_eq!(fixture.state(), case.start_state, "{}: setup", case.name);
        let before = snapshot(&fixture);
        let candidate_revision = revision.as_deref().unwrap_or("sha256:wrong");
        let result: Result<String, String> = match case.operation {
            Operation::BeginReview => fixture
                .store
                .begin_candidate_review(ARTIFACT, candidate_revision)
                .map(|request| request.candidate_revision),
            Operation::Approve | Operation::Reject => fixture
                .store
                .record_candidate_decision(
                    ARTIFACT,
                    candidate_revision,
                    if matches!(case.operation, Operation::Approve) {
                        "approved"
                    } else {
                        "rejected"
                    },
                    "reviewer".into(),
                    None,
                )
                .map(|decision| decision.decision),
            Operation::Accept => fixture
                .store
                .accept_candidate(ARTIFACT, candidate_revision)
                .map(|revision| revision.revision),
            Operation::Replace => {
                let target = if case.start_state == SemanticState::Accepted {
                    Some("sha256:stale".into())
                } else {
                    accepted
                };
                fixture
                    .store
                    .stage_candidate(fixture.identity(target), "# Revised\n")
                    .map(|candidate| candidate.revision)
            }
        };
        match case.expected {
            ExpectedOutcome::Rejected => assert_rejected(&fixture, case.name, result, before),
            ExpectedOutcome::State(expected) => {
                assert!(
                    result.is_ok(),
                    "{}: expected success: {:?}",
                    case.name,
                    result
                );
                assert_eq!(fixture.state(), expected, "{}: next state", case.name);
            }
        }
    }
}

#[test]
fn wrong_revision_transitions_use_real_active_candidates() {
    for case in WRONG_REVISION_TRANSITIONS {
        let fixture = Fixture::new();
        let (revision, _) = setup(&fixture, case.start_state);
        let revision = revision.unwrap();
        let before = snapshot(&fixture);
        let wrong_revision = "sha256:wrong";
        let result: Result<String, String> = match case.operation {
            Operation::BeginReview => fixture
                .store
                .begin_candidate_review(ARTIFACT, wrong_revision)
                .map(|request| request.candidate_revision),
            Operation::Approve => fixture
                .store
                .record_candidate_decision(
                    ARTIFACT,
                    wrong_revision,
                    "approved",
                    "reviewer".into(),
                    None,
                )
                .map(|decision| decision.decision),
            Operation::Accept => fixture
                .store
                .accept_candidate(ARTIFACT, wrong_revision)
                .map(|accepted| accepted.revision),
            _ => unreachable!(),
        };
        assert_rejected(&fixture, case.name, result, before);
        assert_eq!(fixture.state(), case.start_state);
        assert!(!revision.is_empty());
    }
}

#[test]
fn duplicate_and_conflicting_decisions_preserve_original_evidence() {
    for (name, decision) in [
        ("duplicate approval", "approved"),
        ("conflicting rejection", "rejected"),
    ] {
        let fixture = Fixture::new();
        let staged = fixture
            .store
            .stage_candidate(fixture.identity(None), BODY)
            .unwrap();
        fixture
            .store
            .begin_candidate_review(ARTIFACT, &staged.revision)
            .unwrap();
        fixture
            .store
            .record_candidate_decision(
                ARTIFACT,
                &staged.revision,
                "approved",
                "original-reviewer".into(),
                Some("original rationale".into()),
            )
            .unwrap();
        let decision_before = decision_bytes(&fixture, &staged.revision);
        let before = snapshot(&fixture);
        assert_rejected(
            &fixture,
            name,
            fixture.store.record_candidate_decision(
                ARTIFACT,
                &staged.revision,
                decision,
                "second-reviewer".into(),
                Some("replacement rationale".into()),
            ),
            before,
        );
        let decision_after = decision_bytes(&fixture, &staged.revision);
        assert_eq!(
            decision_after, decision_before,
            "{name}: decision evidence changed"
        );
        let record: serde_json::Value = serde_json::from_slice(&decision_after).unwrap();
        assert_eq!(record["decision"], "approved");
        assert_eq!(record["decided_by"], "original-reviewer");
        assert_eq!(record["rationale"], "original rationale");
    }
}

#[test]
fn evidence_corruption_and_stale_source_are_rejected() {
    let fixture = Fixture::new();
    fixture
        .store
        .stage_candidate(fixture.identity(None), BODY)
        .unwrap();
    let path = fixture.dir.join(".rmwm/staged/raw-adc-domain-framing.json");
    let before = snapshot(&fixture);
    let mut record: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    record["content"]["size"] = serde_json::json!(0);
    fs::write(&path, serde_json::to_vec(&record).unwrap()).unwrap();
    assert!(fixture
        .store
        .read_staged_candidate(ARTIFACT, None, None)
        .is_err());
    assert_eq!(
        fs::read(fixture.dir.join("requirements_model.yaml")).unwrap(),
        before.manifest
    );

    let fixture = Fixture::new();
    let staged = fixture
        .store
        .stage_candidate(fixture.identity(None), BODY)
        .unwrap();
    fixture
        .store
        .begin_candidate_review(ARTIFACT, &staged.revision)
        .unwrap();
    let before = snapshot(&fixture);
    fs::write(fixture.dir.join("story.md"), "changed\n").unwrap();
    assert!(fixture
        .store
        .record_candidate_decision(
            ARTIFACT,
            &staged.revision,
            "approved",
            "reviewer".into(),
            None
        )
        .is_err());
    assert_eq!(
        fs::read(fixture.dir.join("requirements_model.yaml")).unwrap(),
        before.manifest
    );
}

#[test]
fn accepted_reacceptance_reports_direct_downstream_impact() {
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/raw-adc");
    let dir = std::env::temp_dir().join(format!(
        "rmwm-review-downstream-{}",
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
    let before = accepted_revision(&store, ARTIFACT);
    let story = accepted_revision(&store, STORY);
    let candidate = store
        .stage_candidate(
            CandidateIdentity {
                model_id: "raw-adc".into(),
                artifact_id: ARTIFACT.into(),
                artifact_type: "domain_framing".into(),
                target_revision: Some(before.clone()),
                source_revisions: BTreeMap::from([(STORY.into(), story.clone())]),
            },
            "# Revised framing\n",
        )
        .unwrap();
    store
        .begin_candidate_review(ARTIFACT, &candidate.revision)
        .unwrap();
    store
        .record_candidate_decision(
            ARTIFACT,
            &candidate.revision,
            "approved",
            "reviewer".into(),
            None,
        )
        .unwrap();
    let accepted = store
        .accept_candidate(ARTIFACT, &candidate.revision)
        .unwrap();
    assert_ne!(accepted.revision, before);
    assert_eq!(accepted.sources.get(STORY), Some(&story));
    let report = store
        .report_affected_downstream_artifacts(ARTIFACT)
        .unwrap();
    let ontology = report
        .affected_artifacts
        .iter()
        .find(|a| a.artifact_id == "raw-adc-domain-ontology")
        .unwrap();
    assert_eq!(ontology.bound_source_revision, before);
    assert_eq!(ontology.current_source_revision, accepted.revision);
    assert_eq!(
        store
            .inspect_model_state()
            .unwrap()
            .artifacts
            .into_iter()
            .find(|a| a.artifact_id == "raw-adc-domain-ontology")
            .unwrap()
            .state,
        "review_required"
    );
    fs::remove_dir_all(dir).unwrap();
}
