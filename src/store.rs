use crate::{
    digest::{content_digest, revision_handle},
    model::{
        AcceptedRevision, ArtifactState, CandidateDecision, CandidateIdentity,
        CandidateReviewRequest, ContentDescriptor, Manifest, ModelState, StagedCandidate,
    },
};
use serde::Deserialize;
use std::{
    collections::BTreeMap,
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub struct ModelStore {
    root: PathBuf,
    manifest_path: PathBuf,
}

impl ModelStore {
    pub fn open(root: impl Into<PathBuf>) -> Self {
        let root = root.into();
        Self {
            manifest_path: root.join("requirements_model.yaml"),
            root,
        }
    }

    pub fn inspect_model_state(&self) -> Result<ModelState, String> {
        let manifest = self.manifest()?;
        let mut artifacts = Vec::new();
        for (artifact_id, descriptor) in &manifest.artifacts {
            let state = match &descriptor.accepted {
                None => "absent",
                Some(accepted) => {
                    let path = self.artifact_path(&descriptor.representation.path)?;
                    if !path.exists() {
                        "absent"
                    } else if self.matches_accepted(&path, accepted)? {
                        "accepted"
                    } else {
                        "modified"
                    }
                }
            };
            let candidate = self.candidate_state(artifact_id)?;
            let (state, candidate_revision) = candidate
                .map(|(state, revision)| (state, Some(revision)))
                .unwrap_or((state, None));
            artifacts.push(ArtifactState {
                artifact_id: artifact_id.clone(),
                descriptor: descriptor.clone(),
                state: state.into(),
                candidate_revision,
            });
        }
        Ok(ModelState {
            model_id: manifest.model_id,
            artifacts,
        })
    }

    pub fn read_accepted_artifact(&self, artifact_id: &str) -> Result<serde_json::Value, String> {
        let manifest = self.manifest()?;
        let descriptor = manifest
            .artifacts
            .get(artifact_id)
            .ok_or_else(|| "unknown artifact".to_owned())?;
        let accepted = descriptor
            .accepted
            .as_ref()
            .ok_or_else(|| "artifact has no accepted revision".to_owned())?;
        let path = self.artifact_path(&descriptor.representation.path)?;
        let bytes = fs::read(&path).map_err(|error| error.to_string())?;
        verify_content(&bytes, &accepted.content)?;
        let text = String::from_utf8(bytes).map_err(|error| error.to_string())?;
        Ok(serde_json::json!({"artifact_id": artifact_id, "text": text, "descriptor": accepted}))
    }

    pub fn begin_candidate(
        &self,
        identity: CandidateIdentity,
    ) -> Result<CandidateIdentity, String> {
        validate_artifact_id(&identity.artifact_id)?;
        let manifest = self.manifest()?;
        if identity.model_id != manifest.model_id {
            return Err("model ID mismatch".into());
        }
        let descriptor = manifest
            .artifacts
            .get(&identity.artifact_id)
            .ok_or_else(|| "unknown artifact".to_owned())?;
        if descriptor.artifact_type != "domain_framing" {
            return Err("only domain framing candidates are supported".into());
        }
        if descriptor.artifact_type != identity.artifact_type {
            return Err("artifact type mismatch".into());
        }
        let current_target = descriptor
            .accepted
            .as_ref()
            .map(|accepted| accepted.revision.clone());
        if identity.target_revision != current_target {
            return Err("stale or incorrect target revision".into());
        }
        if descriptor.accepted.is_some() {
            self.ensure_current(&identity.artifact_id, descriptor)?;
        }
        if identity.artifact_type == "domain_framing" && identity.source_revisions.len() != 1 {
            return Err("domain framing requires exactly one source".into());
        }
        for (source_id, revision) in &identity.source_revisions {
            let source = manifest
                .artifacts
                .get(source_id)
                .ok_or_else(|| format!("unknown source {source_id}"))?;
            if identity.artifact_type == "domain_framing" && source.artifact_type != "system_story"
            {
                return Err("domain framing source must be a system story".into());
            }
            if source.accepted.as_ref().map(|accepted| &accepted.revision) != Some(revision) {
                return Err(format!(
                    "stale or incorrect source revision for {source_id}"
                ));
            }
            self.ensure_current(source_id, source)?;
        }
        Ok(identity)
    }

    pub fn stage_candidate(
        &self,
        identity: CandidateIdentity,
        body: &str,
    ) -> Result<StagedCandidate, String> {
        let identity = self.begin_candidate(identity)?;
        let descriptor = self
            .manifest()?
            .artifacts
            .get(&identity.artifact_id)
            .cloned()
            .ok_or_else(|| "unknown artifact".to_owned())?;
        if descriptor.representation.encoding != "utf-8"
            || descriptor.representation.line_endings != "lf"
        {
            return Err("only UTF-8 LF artifacts are supported".into());
        }
        let body = normalize_body(body)?;
        if body.starts_with("---\n") {
            return Err("body must exclude front matter".into());
        }
        let frontmatter = format!(
            "---\nrmwm:\n  schema: \"artifact/v1\"\n  id: \"{}\"\n  type: \"{}\"\n---\n",
            identity.artifact_id, identity.artifact_type
        );
        let bytes = format!("{frontmatter}{body}").into_bytes();
        let content = ContentDescriptor {
            digest: content_digest(&bytes),
            size: bytes.len(),
        };
        let sources: Vec<_> = identity
            .source_revisions
            .iter()
            .map(|(id, revision)| (id.clone(), revision.clone()))
            .collect();
        let revision = revision_handle(
            &identity.model_id,
            &identity.artifact_id,
            &identity.artifact_type,
            &identity.artifact_type,
            &content.digest,
            &sources,
        );
        let staged = StagedCandidate {
            identity,
            bytes,
            content,
            revision,
            state: "staged".into(),
        };
        self.prepare_staged_dir()?;
        let staged_path = self.staged_path(&staged.identity.artifact_id)?;
        let serialized = serde_json::to_vec(&staged).map_err(|error| error.to_string())?;
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&staged_path)
            .map_err(|error| {
                if error.kind() == std::io::ErrorKind::AlreadyExists {
                    "a staged candidate already exists; replacement is not supported".to_owned()
                } else {
                    error.to_string()
                }
            })?;
        file.write_all(&serialized)
            .map_err(|error| error.to_string())?;
        Ok(staged)
    }

    pub fn read_staged_candidate(&self, artifact_id: &str) -> Result<StagedCandidate, String> {
        self.validated_staged_candidate(artifact_id)
    }

    pub fn begin_candidate_review(
        &self,
        artifact_id: &str,
        candidate_revision: &str,
    ) -> Result<CandidateReviewRequest, String> {
        let candidate = self.matching_staged_candidate(artifact_id, candidate_revision)?;
        self.prepare_review_dir(&candidate.identity.artifact_id)?;
        let path =
            self.review_request_path(&candidate.identity.artifact_id, &candidate.revision)?;
        let request = CandidateReviewRequest {
            artifact_id: candidate.identity.artifact_id,
            candidate_revision: candidate.revision,
        };
        write_new_json(&path, &request, "a review request already exists")?;
        Ok(request)
    }

    pub fn record_candidate_decision(
        &self,
        artifact_id: &str,
        candidate_revision: &str,
        decision: &str,
        decided_by: String,
        rationale: Option<String>,
    ) -> Result<CandidateDecision, String> {
        if decision != "approved" && decision != "rejected" {
            return Err("decision must be approved or rejected".into());
        }
        let candidate = self.matching_staged_candidate(artifact_id, candidate_revision)?;
        let request_path =
            self.review_request_path(&candidate.identity.artifact_id, &candidate.revision)?;
        let request: CandidateReviewRequest = read_json(&request_path, "missing review request")?;
        if request.artifact_id != candidate.identity.artifact_id
            || request.candidate_revision != candidate.revision
        {
            return Err("review request does not match staged candidate".into());
        }
        self.prepare_review_dir(&candidate.identity.artifact_id)?;
        let path =
            self.review_decision_path(&candidate.identity.artifact_id, &candidate.revision)?;
        let record = CandidateDecision {
            artifact_id: candidate.identity.artifact_id,
            candidate_revision: candidate.revision,
            decision: decision.into(),
            decided_by,
            rationale,
        };
        write_new_json(&path, &record, "a candidate decision already exists")?;
        Ok(record)
    }

    fn manifest(&self) -> Result<Manifest, String> {
        serde_yaml::from_slice(&fs::read(&self.manifest_path).map_err(|error| error.to_string())?)
            .map_err(|error| error.to_string())
    }
    fn artifact_path(&self, relative: &str) -> Result<PathBuf, String> {
        let path = Path::new(relative);
        if path.is_absolute()
            || path
                .components()
                .any(|component| matches!(component, std::path::Component::ParentDir))
        {
            return Err("invalid artifact path".into());
        }
        let root = fs::canonicalize(&self.root).map_err(|error| error.to_string())?;
        let joined = root.join(path);
        if joined.exists() {
            let canonical = fs::canonicalize(&joined).map_err(|error| error.to_string())?;
            if !canonical.starts_with(&root) {
                return Err("artifact path escapes model root".into());
            }
            Ok(canonical)
        } else {
            Ok(joined)
        }
    }
    fn matches_accepted(&self, path: &Path, accepted: &AcceptedRevision) -> Result<bool, String> {
        Ok(verify_content(
            &fs::read(path).map_err(|error| error.to_string())?,
            &accepted.content,
        )
        .is_ok())
    }
    fn ensure_current(
        &self,
        artifact_id: &str,
        descriptor: &crate::model::ArtifactDescriptor,
    ) -> Result<(), String> {
        let accepted = descriptor
            .accepted
            .as_ref()
            .ok_or_else(|| format!("source {artifact_id} has no accepted revision"))?;
        let path = self.artifact_path(&descriptor.representation.path)?;
        let bytes =
            fs::read(&path).map_err(|error| format!("cannot read {artifact_id}: {error}"))?;
        verify_content(&bytes, &accepted.content)
            .map_err(|error| format!("stale artifact {artifact_id}: {error}"))
    }
    fn staged_dir(&self) -> Result<PathBuf, String> {
        let root = fs::canonicalize(&self.root).map_err(|error| error.to_string())?;
        let rmwm_path = root.join(".rmwm");
        if !rmwm_path.exists() {
            return Ok(rmwm_path.join("staged"));
        }
        let rmwm = validate_directory(&root, &rmwm_path, "rmwm directory")?;
        let staged = rmwm.join("staged");
        if !staged.exists() {
            return Ok(staged);
        }
        validate_directory(&root, &staged, "staged directory")
    }
    fn prepare_staged_dir(&self) -> Result<(), String> {
        let root = fs::canonicalize(&self.root).map_err(|error| error.to_string())?;
        let rmwm_path = root.join(".rmwm");
        let rmwm = if rmwm_path.exists() {
            validate_directory(&root, &rmwm_path, "rmwm directory")?
        } else {
            fs::create_dir(&rmwm_path).map_err(|error| error.to_string())?;
            rmwm_path
        };
        let staged_path = rmwm.join("staged");
        if staged_path.exists() {
            validate_directory(&root, &staged_path, "staged directory")?;
        } else {
            fs::create_dir(&staged_path).map_err(|error| error.to_string())?;
            validate_directory(&root, &staged_path, "staged directory")?;
        }
        Ok(())
    }
    fn staged_path(&self, artifact_id: &str) -> Result<PathBuf, String> {
        validate_artifact_id(artifact_id)?;
        Ok(self.staged_dir()?.join(format!("{artifact_id}.json")))
    }
    fn validated_staged_candidate(&self, artifact_id: &str) -> Result<StagedCandidate, String> {
        validate_artifact_id(artifact_id)?;
        let candidate: StagedCandidate =
            read_json(&self.staged_path(artifact_id)?, "missing staged candidate")?;
        if candidate.state != "staged" || candidate.identity.artifact_id != artifact_id {
            return Err("invalid staged candidate identity".into());
        }
        verify_content(&candidate.bytes, &candidate.content)?;
        let sources: Vec<_> = candidate
            .identity
            .source_revisions
            .iter()
            .map(|(id, revision)| (id.clone(), revision.clone()))
            .collect();
        let revision = revision_handle(
            &candidate.identity.model_id,
            &candidate.identity.artifact_id,
            &candidate.identity.artifact_type,
            &candidate.identity.artifact_type,
            &candidate.content.digest,
            &sources,
        );
        if candidate.revision != revision {
            return Err("staged candidate revision mismatch".into());
        }
        self.begin_candidate(candidate.identity.clone())?;
        Ok(candidate)
    }
    fn matching_staged_candidate(
        &self,
        artifact_id: &str,
        revision: &str,
    ) -> Result<StagedCandidate, String> {
        let candidate = self.validated_staged_candidate(artifact_id)?;
        if candidate.revision != revision {
            return Err("candidate revision does not match staged candidate".into());
        }
        Ok(candidate)
    }
    fn review_dir(&self, artifact_id: &str) -> Result<PathBuf, String> {
        validate_artifact_id(artifact_id)?;
        let root = fs::canonicalize(&self.root).map_err(|error| error.to_string())?;
        let rmwm = validate_directory(&root, &root.join(".rmwm"), "rmwm directory")?;
        let reviews = validate_directory(&root, &rmwm.join("reviews"), "reviews directory")?;
        validate_directory(
            &root,
            &reviews.join(artifact_id),
            "review artifact directory",
        )
    }
    fn prepare_review_dir(&self, artifact_id: &str) -> Result<(), String> {
        self.prepare_staged_dir()?;
        let root = fs::canonicalize(&self.root).map_err(|error| error.to_string())?;
        let rmwm = validate_directory(&root, &root.join(".rmwm"), "rmwm directory")?;
        let reviews = rmwm.join("reviews");
        if reviews.exists() {
            validate_directory(&root, &reviews, "reviews directory")?;
        } else {
            fs::create_dir(&reviews).map_err(|error| error.to_string())?;
        }
        let artifact = reviews.join(artifact_id);
        if artifact.exists() {
            validate_directory(&root, &artifact, "review artifact directory")?;
        } else {
            fs::create_dir(&artifact).map_err(|error| error.to_string())?;
        }
        Ok(())
    }
    fn review_request_path(&self, artifact_id: &str, revision: &str) -> Result<PathBuf, String> {
        Ok(self
            .review_dir(artifact_id)?
            .join(format!("{revision}.request.json")))
    }
    fn review_decision_path(&self, artifact_id: &str, revision: &str) -> Result<PathBuf, String> {
        Ok(self
            .review_dir(artifact_id)?
            .join(format!("{revision}.decision.json")))
    }
    fn candidate_state(&self, artifact_id: &str) -> Result<Option<(&'static str, String)>, String> {
        let staged = self.staged_path(artifact_id)?;
        if !staged.exists() {
            return Ok(None);
        }
        let candidate = self.validated_staged_candidate(artifact_id)?;
        let root = fs::canonicalize(&self.root).map_err(|error| error.to_string())?;
        let rmwm_path = root.join(".rmwm");
        if !rmwm_path.exists() {
            return Ok(Some(("staged", candidate.revision)));
        }
        let rmwm = validate_directory(&root, &rmwm_path, "rmwm directory")?;
        let reviews_path = rmwm.join("reviews");
        if !reviews_path.exists() {
            return Ok(Some(("staged", candidate.revision)));
        }
        let reviews = validate_directory(&root, &reviews_path, "reviews directory")?;
        let artifact_path = reviews.join(&candidate.identity.artifact_id);
        if !artifact_path.exists() {
            return Ok(Some(("staged", candidate.revision)));
        }
        let artifact_dir = validate_directory(&root, &artifact_path, "review artifact directory")?;
        let decision_path = artifact_dir.join(format!("{}.decision.json", candidate.revision));
        if decision_path.exists() {
            let request_path = artifact_dir.join(format!("{}.request.json", candidate.revision));
            let request: CandidateReviewRequest =
                read_json(&request_path, "missing review request")?;
            if request.artifact_id != candidate.identity.artifact_id
                || request.candidate_revision != candidate.revision
            {
                return Err("review request does not match staged candidate".into());
            }
            let decision: CandidateDecision =
                read_json(&decision_path, "invalid candidate decision")?;
            if decision.artifact_id != candidate.identity.artifact_id
                || decision.candidate_revision != candidate.revision
            {
                return Err("candidate decision does not match staged candidate".into());
            }
            return match decision.decision.as_str() {
                "approved" => Ok(Some(("approved", candidate.revision))),
                "rejected" => Ok(Some(("rejected", candidate.revision))),
                _ => Err("invalid candidate decision".into()),
            };
        }
        let request_path = artifact_dir.join(format!("{}.request.json", candidate.revision));
        if request_path.exists() {
            let request: CandidateReviewRequest =
                read_json(&request_path, "invalid review request")?;
            if request.artifact_id != candidate.identity.artifact_id
                || request.candidate_revision != candidate.revision
            {
                return Err("review request does not match staged candidate".into());
            }
            return Ok(Some(("under_review", candidate.revision)));
        }
        Ok(Some(("staged", candidate.revision)))
    }
}

fn read_json<T: serde::de::DeserializeOwned>(path: &Path, missing: &str) -> Result<T, String> {
    let metadata = fs::symlink_metadata(path).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            missing.to_owned()
        } else {
            error.to_string()
        }
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(format!(
            "persisted record is not a regular file: {}",
            path.display()
        ));
    }
    let bytes = fs::read(path).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            missing.into()
        } else {
            error.to_string()
        }
    })?;
    serde_json::from_slice(&bytes).map_err(|error| error.to_string())
}

fn write_new_json<T: serde::Serialize>(path: &Path, value: &T, exists: &str) -> Result<(), String> {
    let bytes = serde_json::to_vec(value).map_err(|error| error.to_string())?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::AlreadyExists {
                exists.into()
            } else {
                error.to_string()
            }
        })?;
    file.write_all(&bytes).map_err(|error| error.to_string())
}

fn validate_directory(root: &Path, path: &Path, name: &str) -> Result<PathBuf, String> {
    let metadata = fs::symlink_metadata(path).map_err(|error| error.to_string())?;
    if metadata.file_type().is_symlink() {
        return Err(format!("{name} escapes model root"));
    }
    if !metadata.is_dir() {
        return Err(format!("{name} is not a directory"));
    }
    let canonical = fs::canonicalize(path).map_err(|error| error.to_string())?;
    if !canonical.starts_with(root) {
        return Err(format!("{name} escapes model root"));
    }
    Ok(canonical)
}

fn validate_artifact_id(artifact_id: &str) -> Result<(), String> {
    let mut characters = artifact_id.chars();
    let valid_first = characters
        .next()
        .is_some_and(|character| character.is_ascii_alphanumeric());
    if !valid_first
        || !characters.all(|character| {
            character.is_ascii_alphanumeric() || character == '-' || character == '_'
        })
    {
        return Err("invalid artifact ID for staged path".into());
    }
    Ok(())
}

fn verify_content(bytes: &[u8], expected: &ContentDescriptor) -> Result<(), String> {
    if bytes.len() != expected.size {
        return Err("accepted artifact byte count mismatch".into());
    }
    if content_digest(bytes) != expected.digest {
        return Err("accepted artifact digest mismatch".into());
    }
    Ok(())
}

fn normalize_body(body: &str) -> Result<String, String> {
    if body.contains('\u{0000}') {
        return Err("candidate body contains NUL".into());
    }
    let body = body.replace("\r\n", "\n").replace('\r', "\n");
    Ok(if body.ends_with('\n') {
        body
    } else {
        format!("{body}\n")
    })
}

#[derive(Debug, Deserialize)]
pub struct CandidateRequest {
    pub model_id: String,
    pub artifact_id: String,
    pub artifact_type: String,
    pub target_revision: Option<String>,
    #[serde(default)]
    pub source_revisions: BTreeMap<String, String>,
}

impl From<CandidateRequest> for CandidateIdentity {
    fn from(request: CandidateRequest) -> Self {
        Self {
            model_id: request.model_id,
            artifact_id: request.artifact_id,
            artifact_type: request.artifact_type,
            target_revision: request.target_revision,
            source_revisions: request.source_revisions,
        }
    }
}
