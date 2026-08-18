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
pub struct ArtifactState {
    pub artifact_id: String,
    #[serde(flatten)]
    pub descriptor: ArtifactDescriptor,
    pub state: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModelState {
    pub model_id: String,
    pub artifacts: Vec<ArtifactState>,
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
    pub state: String,
}
