use requirements_model_workflow_mcp::{model::CandidateIdentity, store::ModelStore};
use std::{
    collections::BTreeMap,
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

const VOCABULARY_ARTIFACT: &str = "raw-adc-controlled-vocabulary";
const ONTOLOGY_ARTIFACT: &str = "raw-adc-domain-ontology";
const FRAMING_ARTIFACT: &str = "raw-adc-domain-framing";

const ONTOLOGY_BODY: &str = "# Ontology\n\n## Concepts\n\n| ID | Concept |\n| --- | --- |\n| `concept.c001` | Capture |\n\n## Properties\n\n| ID | Property |\n| --- | --- |\n| `property.p001` | Capture identity |\n\n## Relationships\n\n| ID | Relationship |\n| --- | --- |\n| `relationship.r001` | relates to |\n\n## Constraints\n\n* `constraint.k001` A constraint.\n";

fn fixture() -> (PathBuf, ModelStore, String, String, String) {
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/raw-adc");
    let dir = std::env::temp_dir().join(format!(
        "rmwm-vocab-{}",
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
    manifest.push_str(concat!(
        "\n  raw-adc-controlled-vocabulary:\n",
        "    type: \"controlled_vocabulary\"\n",
        "    representation:\n",
        "      path: \"controlled_vocabulary.md\"\n",
        "      media_type: \"text/markdown\"\n",
        "      encoding: \"utf-8\"\n",
        "      line_endings: \"lf\"\n",
        "    accepted: null\n",
    ));
    fs::write(dir.join("requirements_model.yaml"), manifest).unwrap();
    let store = ModelStore::open(&dir);
    let state = store.inspect_model_state().unwrap();
    let revision_of = |artifact_id: &str| {
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
    let story_revision = revision_of("raw-adc-story");
    let framing_revision = revision_of(FRAMING_ARTIFACT);
    let ontology_revision = revision_of(ONTOLOGY_ARTIFACT);
    (
        dir,
        store,
        story_revision,
        framing_revision,
        ontology_revision,
    )
}

fn vocabulary_identity(
    target_revision: Option<String>,
    ontology_revision: String,
) -> CandidateIdentity {
    CandidateIdentity {
        model_id: "raw-adc".into(),
        artifact_id: VOCABULARY_ARTIFACT.into(),
        artifact_type: "controlled_vocabulary".into(),
        target_revision,
        source_revisions: BTreeMap::from([(ONTOLOGY_ARTIFACT.into(), ontology_revision)]),
    }
}

fn ontology_identity(target_revision: String, framing_revision: String) -> CandidateIdentity {
    CandidateIdentity {
        model_id: "raw-adc".into(),
        artifact_id: ONTOLOGY_ARTIFACT.into(),
        artifact_type: "domain_ontology".into(),
        target_revision: Some(target_revision),
        source_revisions: BTreeMap::from([(FRAMING_ARTIFACT.into(), framing_revision)]),
    }
}

fn accepted_revision(store: &ModelStore, artifact_id: &str) -> String {
    store
        .inspect_model_state()
        .unwrap()
        .artifacts
        .into_iter()
        .find(|artifact| artifact.artifact_id == artifact_id)
        .unwrap()
        .descriptor
        .accepted
        .unwrap()
        .revision
}

fn stage_review_approve_accept(
    store: &ModelStore,
    identity: CandidateIdentity,
    body: &str,
) -> requirements_model_workflow_mcp::model::AcceptedRevision {
    let artifact_id = identity.artifact_id.clone();
    store.begin_candidate(identity.clone()).unwrap();
    let candidate = store.stage_candidate(identity, body).unwrap();
    store
        .begin_candidate_review(&artifact_id, &candidate.revision)
        .unwrap();
    store
        .record_candidate_decision(
            &artifact_id,
            &candidate.revision,
            "approved",
            "reviewer".into(),
            None,
        )
        .unwrap();
    store
        .accept_candidate(&artifact_id, &candidate.revision)
        .unwrap()
}

#[test]
fn vocabulary_rebinds_to_new_ontology_after_ontology_reacceptance() {
    let (dir, store, _story_revision, framing_revision, ontology_o1) = fixture();

    // V1 bound to O1.
    let v1 = stage_review_approve_accept(
        &store,
        vocabulary_identity(None, ontology_o1.clone()),
        "# Vocabulary V1",
    );
    assert_eq!(
        v1.sources.get(ONTOLOGY_ARTIFACT),
        Some(&ontology_o1.clone())
    );

    // Accept ontology O2.
    stage_review_approve_accept(
        &store,
        ontology_identity(ontology_o1.clone(), framing_revision),
        ONTOLOGY_BODY,
    );
    let ontology_o2 = accepted_revision(&store, ONTOLOGY_ARTIFACT);
    assert_ne!(ontology_o1, ontology_o2);

    // V1 becomes review_required because its bound ontology source is stale.
    assert_eq!(
        store
            .inspect_model_state()
            .unwrap()
            .artifacts
            .into_iter()
            .find(|artifact| artifact.artifact_id == VOCABULARY_ARTIFACT)
            .unwrap()
            .state,
        "review_required"
    );

    // Begin, stage, review, approve and accept V2 against O2.
    let v1_revision = accepted_revision(&store, VOCABULARY_ARTIFACT);
    let v2 = stage_review_approve_accept(
        &store,
        vocabulary_identity(Some(v1_revision), ontology_o2.clone()),
        "# Vocabulary V2",
    );
    assert_eq!(v2.sources.get(ONTOLOGY_ARTIFACT), Some(&ontology_o2));

    assert_eq!(
        store
            .inspect_model_state()
            .unwrap()
            .artifacts
            .into_iter()
            .find(|artifact| artifact.artifact_id == VOCABULARY_ARTIFACT)
            .unwrap()
            .state,
        "accepted"
    );
    assert!(store
        .report_affected_downstream_artifacts(ONTOLOGY_ARTIFACT)
        .unwrap()
        .affected_artifacts
        .is_empty());
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn vocabulary_candidate_requires_exactly_one_source() {
    let (dir, store, _story_revision, _framing_revision, ontology_revision) = fixture();
    let mut missing = vocabulary_identity(None, ontology_revision.clone());
    missing.source_revisions.clear();
    assert!(store
        .begin_candidate(missing)
        .unwrap_err()
        .contains("exactly one source"));

    let mut multiple = vocabulary_identity(None, ontology_revision);
    multiple
        .source_revisions
        .insert(FRAMING_ARTIFACT.into(), "sha256:extra".into());
    assert!(store
        .begin_candidate(multiple)
        .unwrap_err()
        .contains("exactly one source"));
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn vocabulary_candidate_rejects_wrong_source_artifact_type() {
    let (dir, store, _story_revision, framing_revision, _ontology_revision) = fixture();
    let mut identity = vocabulary_identity(None, framing_revision.clone());
    identity.source_revisions.clear();
    identity
        .source_revisions
        .insert(FRAMING_ARTIFACT.into(), framing_revision);
    assert!(store
        .begin_candidate(identity)
        .unwrap_err()
        .contains("controlled vocabulary source must be a domain ontology"));
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn vocabulary_candidate_rejects_stale_ontology_source_revision() {
    let (dir, store, _story_revision, _framing_revision, _ontology_revision) = fixture();
    let identity = vocabulary_identity(None, "sha256:stale".into());
    assert!(store
        .begin_candidate(identity)
        .unwrap_err()
        .contains("stale or incorrect source revision"));
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn vocabulary_candidate_rejects_artifact_type_mismatch() {
    let (dir, store, _story_revision, _framing_revision, ontology_revision) = fixture();
    let mut identity = vocabulary_identity(None, ontology_revision);
    identity.artifact_type = "domain_ontology".into();
    assert!(store
        .begin_candidate(identity)
        .unwrap_err()
        .contains("artifact type mismatch"));
    fs::remove_dir_all(dir).unwrap();
}
