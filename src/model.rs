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
