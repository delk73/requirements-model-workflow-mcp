use crate::{
    digest::{content_digest, revision_handle},
    model::{
        AcceptedRevision, ArtifactState, CandidateIdentity, ContentDescriptor, Manifest,
        ModelState, StagedCandidate,
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
            let state = if self.staged_path(artifact_id)?.exists() {
                "staged"
            } else {
                state
            };
            artifacts.push(ArtifactState {
                artifact_id: artifact_id.clone(),
                descriptor: descriptor.clone(),
                state: state.into(),
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
        Ok(serde_json::json!({"artifact_id": artifact_id, "bytes": bytes, "descriptor": accepted}))
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
        fs::create_dir_all(self.root.join(".rmwm/staged")).map_err(|error| error.to_string())?;
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
        let staged = root.join(".rmwm/staged");
        if !staged.exists() {
            return Ok(staged);
        }
        let canonical = fs::canonicalize(&staged).map_err(|error| error.to_string())?;
        if !canonical.starts_with(&root) {
            return Err("staged directory escapes model root".into());
        }
        Ok(canonical)
    }
    fn staged_path(&self, artifact_id: &str) -> Result<PathBuf, String> {
        validate_artifact_id(artifact_id)?;
        Ok(self.staged_dir()?.join(format!("{artifact_id}.json")))
    }
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
