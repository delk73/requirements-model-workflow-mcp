use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Manifest {
    pub schema: String,
    pub model_id: String,
    pub artifacts: BTreeMap<String, ArtifactDescriptor>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ArtifactDescriptor {
    #[serde(rename = "type")]
    pub artifact_type: String,
    pub representation: Representation,
    pub accepted: Option<AcceptedRevision>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Representation {
    pub path: String,
    pub media_type: String,
    pub encoding: String,
    pub line_endings: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AcceptedRevision {
    pub revision: String,
    pub content: ContentDescriptor,
    #[serde(default)]
    pub sources: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct ContentDescriptor {
    pub digest: String,
    pub size: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OntologyElementKind {
    Concept,
    Property,
    Relationship,
    Constraint,
}

impl OntologyElementKind {
    pub fn section_name(self) -> &'static str {
        match self {
            Self::Concept => "Concepts",
            Self::Property => "Properties",
            Self::Relationship => "Relationships",
            Self::Constraint => "Constraints",
        }
    }

    pub fn id_prefix(self) -> &'static str {
        match self {
            Self::Concept => "concept.c",
            Self::Property => "property.p",
            Self::Relationship => "relationship.r",
            Self::Constraint => "constraint.k",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OntologyElement {
    pub id: String,
    pub kind: OntologyElementKind,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OntologyElementIndex {
    pub elements: Vec<OntologyElement>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VocabularyOntologyReference {
    pub ontology_element_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Requirement {
    pub id: String,
    pub prose: String,
    pub ontology_element_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequirementIndex {
    pub requirements: Vec<Requirement>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ImplementationTarget {
    pub id: String,
    pub repository_revision: String,
    pub path: String,
    pub symbol: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImplementationIndex {
    pub targets: Vec<ImplementationTarget>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TraceLink {
    pub source_artifact_id: String,
    pub source_element_id: String,
    pub relationship: String,
    pub target_artifact_id: String,
    pub target_element_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceabilityIndex {
    pub links: Vec<TraceLink>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequirementDecompositionIndex {
    pub parents: Vec<RequirementDecomposition>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequirementDecomposition {
    pub parent_requirement_id: String,
    pub outcome: RequirementDecompositionOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RequirementDecompositionOutcome {
    Children {
        child_requirement_ids: Vec<String>,
        ontology_basis_ids: Vec<String>,
        rationale: Option<String>,
    },
    NoFurtherDecomposition,
}

#[derive(Debug, Clone, Serialize)]
pub struct AcceptedArtifactRead {
    pub artifact_id: String,
    pub text: String,
    pub descriptor: AcceptedRevision,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_lines: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_line: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_line: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ArtifactState {
    pub artifact_id: String,
    #[serde(flatten)]
    pub descriptor: ArtifactDescriptor,
    pub state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub candidate_revision: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModelState {
    pub model_id: String,
    pub artifacts: Vec<ArtifactState>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct AffectedDownstreamArtifact {
    pub artifact_id: String,
    pub bound_source_revision: String,
    pub current_source_revision: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AffectedDownstreamArtifactsReport {
    pub artifact_id: String,
    pub accepted_revision: String,
    pub affected_artifacts: Vec<AffectedDownstreamArtifact>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct CandidateIdentity {
    pub model_id: String,
    pub artifact_id: String,
    pub artifact_type: String,
    pub target_revision: Option<String>,
    pub source_revisions: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StagedCandidate {
    pub identity: CandidateIdentity,
    pub bytes: Vec<u8>,
    pub content: ContentDescriptor,
    pub revision: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supersedes: Option<String>,
    pub state: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct StagedCandidateView {
    pub identity: CandidateIdentity,
    pub text: String,
    pub content: ContentDescriptor,
    pub revision: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supersedes: Option<String>,
    pub state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_lines: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_line: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_line: Option<usize>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CandidateReviewRequest {
    pub artifact_id: String,
    pub candidate_revision: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CandidateDecision {
    pub artifact_id: String,
    pub candidate_revision: String,
    pub decision: String,
    pub decided_by: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rationale: Option<String>,
}
