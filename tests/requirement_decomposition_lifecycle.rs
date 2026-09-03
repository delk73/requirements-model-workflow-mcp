use requirements_model_workflow_mcp::{model::CandidateIdentity, store::ModelStore};
use std::{
    collections::BTreeMap,
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

const ONTOLOGY: &str = "# Ontology\n\n## Concepts\n\n| ID | Concept |\n| --- | --- |\n| `concept.c001` | Capture |\n| `concept.c002` | State |\n| `concept.c003` | Retention |\n";
const VOCABULARY: &str = "# Vocabulary\n\n## Entries\n\n| Ontology element | Preferred term | Definition |\n| --- | --- | --- |\n| `concept.c001` | Capture | A capture. |\n| `concept.c002` | State | A state. |\n";
const REQUIREMENTS: &str = "# Requirements\n\n## Requirements\n\n### `requirement.r001`\n\nThe system shall identify captures.\n\n#### Ontology elements\n\n| Ontology element |\n| --- |\n| `concept.c001` |\n\n### `requirement.r002`\n\nThe system shall identify capture state.\n\n#### Ontology elements\n\n| Ontology element |\n| --- |\n| `concept.c002` |\n\n### `requirement.r003`\n\nThe system shall retain identified captures.\n\n#### Ontology elements\n\n| Ontology element |\n| --- |\n| `concept.c001` |\n";
const DECOMPOSITION: &str = "# Requirement Decomposition\n\n## Decomposition\n\n### `requirement.r001`\n\n#### Child requirements\n\n- `requirement.r002`\n- `requirement.r003`\n\n#### Ontology basis\n\n- `concept.c002`\n\n#### Rationale\n\nSeparates capture identity from state.\n\n### `requirement.r002`\n\n#### No further decomposition\n\n### `requirement.r003`\n\n#### No further decomposition\n";

fn setup() -> (PathBuf, ModelStore, String, String) {
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/raw-adc");
    let dir = std::env::temp_dir().join(format!(
        "rmwm-decomposition-{}",
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
    manifest.push_str("\n  vocabulary:\n    type: \"controlled_vocabulary\"\n    representation:\n      path: \"vocabulary.md\"\n      media_type: \"text/markdown\"\n      encoding: \"utf-8\"\n      line_endings: \"lf\"\n    accepted: null\n  requirements:\n    type: \"requirements\"\n    representation:\n      path: \"requirements.md\"\n      media_type: \"text/markdown\"\n      encoding: \"utf-8\"\n      line_endings: \"lf\"\n    accepted: null\n  decomposition:\n    type: \"requirement_decomposition\"\n    representation:\n      path: \"decomposition.md\"\n      media_type: \"text/markdown\"\n      encoding: \"utf-8\"\n      line_endings: \"lf\"\n    accepted: null\n");
    fs::write(dir.join("requirements_model.yaml"), manifest).unwrap();
    let store = ModelStore::open(&dir);
    let state = store.inspect_model_state().unwrap();
    let revision = |id: &str| {
        state
            .artifacts
            .iter()
            .find(|a| a.artifact_id == id)
            .unwrap()
            .descriptor
            .accepted
            .as_ref()
            .unwrap()
            .revision
            .clone()
    };
    (
        dir,
        store,
        revision("raw-adc-domain-framing"),
        revision("raw-adc-domain-ontology"),
    )
}
fn identity(
    id: &str,
    artifact_type: &str,
    target: Option<String>,
    source: (&str, String),
) -> CandidateIdentity {
    CandidateIdentity {
        model_id: "raw-adc".into(),
        artifact_id: id.into(),
        artifact_type: artifact_type.into(),
        target_revision: target,
        source_revisions: BTreeMap::from([(source.0.into(), source.1)]),
    }
}
fn accept(store: &ModelStore, candidate: CandidateIdentity, body: &str) {
    let id = candidate.artifact_id.clone();
    let staged = store.stage_candidate(candidate, body).unwrap();
    store.begin_candidate_review(&id, &staged.revision).unwrap();
    store
        .record_candidate_decision(&id, &staged.revision, "approved", "test".into(), None)
        .unwrap();
    store.accept_candidate(&id, &staged.revision).unwrap();
}
fn accepted_revision(store: &ModelStore, id: &str) -> String {
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

#[test]
fn decomposition_validates_graph_basis_binding_and_lifecycle() {
    let (dir, store, framing, ontology_revision) = setup();
    let ontology = ONTOLOGY.replace(
        "| `concept.c003` | Retention |",
        "| `concept.c004` | Retention |",
    );
    let vocabulary = VOCABULARY.replace(
        "| `concept.c002` | State |",
        "| `concept.c002` | State |\n| `concept.c004` | Retention |",
    );
    let requirements = REQUIREMENTS.replace(
        "### `requirement.r003`\n\nThe system shall retain identified captures.\n\n#### Ontology elements\n\n| Ontology element |\n| --- |\n| `concept.c001` |",
        "### `requirement.r003`\n\nThe system shall retain identified captures.\n\n#### Ontology elements\n\n| Ontology element |\n| --- |\n| `concept.c004` |",
    );
    accept(
        &store,
        identity(
            "raw-adc-domain-ontology",
            "domain_ontology",
            Some(ontology_revision),
            ("raw-adc-domain-framing", framing),
        ),
        &ontology,
    );
    let ontology_revision = accepted_revision(&store, "raw-adc-domain-ontology");
    accept(
        &store,
        identity(
            "vocabulary",
            "controlled_vocabulary",
            None,
            ("raw-adc-domain-ontology", ontology_revision),
        ),
        &vocabulary,
    );
    let vocabulary_revision = accepted_revision(&store, "vocabulary");
    accept(
        &store,
        identity(
            "requirements",
            "requirements",
            None,
            ("vocabulary", vocabulary_revision.clone()),
        ),
        &requirements,
    );
    let requirements_revision = accepted_revision(&store, "requirements");
    let candidate = identity(
        "decomposition",
        "requirement_decomposition",
        None,
        ("requirements", requirements_revision.clone()),
    );
    let decomposition =
        DECOMPOSITION.replace("- `concept.c002`", "- `concept.c002`\n- `concept.c004`");
    let mut zero = candidate.clone();
    zero.source_revisions.clear();
    assert!(store
        .begin_candidate(zero)
        .unwrap_err()
        .contains("exactly one source"));
    let mut multiple = candidate.clone();
    multiple
        .source_revisions
        .insert("vocabulary".into(), vocabulary_revision.clone());
    assert!(store
        .begin_candidate(multiple)
        .unwrap_err()
        .contains("exactly one source"));
    let unknown_parent = decomposition.replace("requirement.r001`", "requirement.r999`");
    assert!(store
        .stage_candidate(candidate.clone(), &unknown_parent)
        .unwrap_err()
        .contains("unknown parent"));

    let outside = decomposition.replace("- `concept.c002`", "- `concept.c999`");
    assert!(store
        .stage_candidate(candidate.clone(), &outside)
        .unwrap_err()
        .contains("not represented"));
    let unknown = decomposition.replace("requirement.r003", "requirement.r999");
    assert!(store
        .stage_candidate(candidate.clone(), &unknown)
        .unwrap_err()
        .contains("unknown child"));
    let self_link = decomposition.replace(
        "requirement.r002`\n- `requirement.r003",
        "requirement.r001`\n- `requirement.r003",
    );
    let self_link = self_link.replace(
        "#### Ontology basis\n\n- `concept.c002`",
        "#### Ontology basis\n\n- `concept.c001`",
    );
    let self_link_error = store
        .stage_candidate(candidate.clone(), &self_link)
        .unwrap_err();
    assert!(self_link_error.contains("self-link"), "{self_link_error}");
    let incomplete = decomposition.replace(
        "\n### `requirement.r003`\n\n#### No further decomposition\n",
        "",
    );
    assert!(store
        .stage_candidate(candidate.clone(), &incomplete)
        .unwrap_err()
        .contains("cover"));
    let cycle = DECOMPOSITION.replace("### `requirement.r002`\n\n#### No further decomposition", "### `requirement.r002`\n\n#### Child requirements\n\n- `requirement.r001`\n\n#### Ontology basis\n\n- `concept.c001`");
    assert!(store
        .stage_candidate(candidate.clone(), &cycle)
        .unwrap_err()
        .contains("cycle"));
    let mut wrong = candidate.clone();
    wrong.source_revisions = BTreeMap::from([("vocabulary".into(), vocabulary_revision.clone())]);
    assert!(store
        .begin_candidate(wrong)
        .unwrap_err()
        .contains("requirements"));
    let mut stale = candidate.clone();
    stale.source_revisions = BTreeMap::from([("requirements".into(), "sha256:stale".into())]);
    assert!(store.begin_candidate(stale).unwrap_err().contains("stale"));
    accept(&store, candidate, &decomposition);
    let mut changed = requirements.replace(
        "retain identified captures",
        "retain all identified captures",
    );
    changed.push('\n');
    let req = identity(
        "requirements",
        "requirements",
        Some(requirements_revision),
        ("vocabulary", accepted_revision(&store, "vocabulary")),
    );
    accept(&store, req, &changed);
    assert_eq!(
        store
            .inspect_model_state()
            .unwrap()
            .artifacts
            .into_iter()
            .find(|a| a.artifact_id == "decomposition")
            .unwrap()
            .state,
        "review_required"
    );
    fs::remove_dir_all(dir).unwrap();
}
