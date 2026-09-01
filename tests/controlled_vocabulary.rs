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
const VOCABULARY_BODY: &str = "# Vocabulary\n\n## Entries\n\n| Ontology element | Preferred term | Definition |\n| --- | --- | --- |\n| `concept.c001` | Capture | A grouping. |\n| `property.p001` | Capture identity | An identity. |\n| `relationship.r001` | Relates to | A relation. |\n| `constraint.k001` | Constraint | A rule. |\n";

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

    // Replace the accepted legacy ontology with an ID-bearing revision.
    stage_review_approve_accept(
        &store,
        ontology_identity(ontology_o1.clone(), framing_revision.clone()),
        ONTOLOGY_BODY,
    );
    let ontology_o2 = accepted_revision(&store, ONTOLOGY_ARTIFACT);

    // V1 bound to the current ontology.
    let v1 = stage_review_approve_accept(
        &store,
        vocabulary_identity(None, ontology_o2.clone()),
        VOCABULARY_BODY,
    );
    assert_eq!(
        v1.sources.get(ONTOLOGY_ARTIFACT),
        Some(&ontology_o2.clone())
    );

    // Accept ontology O3.
    stage_review_approve_accept(
        &store,
        ontology_identity(ontology_o2.clone(), framing_revision),
        &format!("{ONTOLOGY_BODY}\n"),
    );
    let ontology_o3 = accepted_revision(&store, ONTOLOGY_ARTIFACT);
    assert_ne!(ontology_o2, ontology_o3);

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

    // Begin, stage, review, approve and accept V2 against O3.
    let v1_revision = accepted_revision(&store, VOCABULARY_ARTIFACT);
    let v2 = stage_review_approve_accept(
        &store,
        vocabulary_identity(Some(v1_revision), ontology_o3.clone()),
        VOCABULARY_BODY,
    );
    assert_eq!(v2.sources.get(ONTOLOGY_ARTIFACT), Some(&ontology_o3));

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
fn vocabulary_candidate_accepts_all_stable_ontology_element_kinds() {
    let (dir, store, _story_revision, framing_revision, ontology_revision) = fixture();
    stage_review_approve_accept(
        &store,
        ontology_identity(ontology_revision.clone(), framing_revision),
        ONTOLOGY_BODY,
    );
    stage_review_approve_accept(
        &store,
        vocabulary_identity(None, accepted_revision(&store, ONTOLOGY_ARTIFACT)),
        VOCABULARY_BODY,
    );
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn vocabulary_candidate_rejects_missing_reference() {
    let (dir, store, _story_revision, framing_revision, ontology_revision) = fixture();
    stage_review_approve_accept(
        &store,
        ontology_identity(ontology_revision, framing_revision),
        ONTOLOGY_BODY,
    );
    let body = VOCABULARY_BODY.replace("| `concept.c001` |", "|  |");
    let error = store
        .stage_candidate(
            vocabulary_identity(None, accepted_revision(&store, ONTOLOGY_ARTIFACT)),
            &body,
        )
        .unwrap_err();
    assert!(error.contains("missing ontology reference"));
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn vocabulary_candidate_rejects_malformed_reference() {
    let (dir, store, _story_revision, framing_revision, ontology_revision) = fixture();
    stage_review_approve_accept(
        &store,
        ontology_identity(ontology_revision, framing_revision),
        ONTOLOGY_BODY,
    );
    let body = VOCABULARY_BODY.replace("concept.c001", "concept.c01");
    let error = store
        .stage_candidate(
            vocabulary_identity(None, accepted_revision(&store, ONTOLOGY_ARTIFACT)),
            &body,
        )
        .unwrap_err();
    assert!(error.contains("malformed ontology reference"));
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn vocabulary_candidate_rejects_unresolved_reference() {
    let (dir, store, _story_revision, framing_revision, ontology_revision) = fixture();
    stage_review_approve_accept(
        &store,
        ontology_identity(ontology_revision, framing_revision),
        ONTOLOGY_BODY,
    );
    let body = VOCABULARY_BODY.replace("concept.c001", "concept.c999");
    let error = store
        .stage_candidate(
            vocabulary_identity(None, accepted_revision(&store, ONTOLOGY_ARTIFACT)),
            &body,
        )
        .unwrap_err();
    assert!(error.contains("unresolved ontology reference: concept.c999"));
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn vocabulary_candidate_rejects_non_current_ontology_source() {
    let (dir, store, _story_revision, framing_revision, ontology_revision) = fixture();
    stage_review_approve_accept(
        &store,
        ontology_identity(ontology_revision.clone(), framing_revision.clone()),
        ONTOLOGY_BODY,
    );
    let current = accepted_revision(&store, ONTOLOGY_ARTIFACT);
    stage_review_approve_accept(
        &store,
        ontology_identity(current.clone(), framing_revision),
        &format!("{ONTOLOGY_BODY}\n"),
    );
    let error = store
        .begin_candidate(vocabulary_identity(None, current))
        .unwrap_err();
    assert!(error.contains("stale or incorrect source revision"));
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn vocabulary_candidate_rejects_legacy_ontology_without_stable_index() {
    let (dir, store, _story_revision, _framing_revision, ontology_revision) = fixture();
    let error = store
        .stage_candidate(
            vocabulary_identity(None, ontology_revision),
            VOCABULARY_BODY,
        )
        .unwrap_err();
    assert!(error.contains("bound ontology cannot provide required stable ontology-element index"));
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
