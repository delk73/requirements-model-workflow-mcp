use crate::{
    digest::{content_digest, revision_handle},
    model::{
        AcceptedRevision, ArtifactState, CandidateDecision, CandidateIdentity,
        CandidateReviewRequest, ContentDescriptor, Manifest, ModelState, StagedCandidate,
    },
};
use fs2::FileExt;
use serde::Deserialize;
use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

#[derive(Deserialize, serde::Serialize)]
struct AcceptanceJournal {
    artifact_id: String,
    candidate_revision: String,
    artifact_path: String,
    old_artifact: Option<Vec<u8>>,
    old_manifest: Vec<u8>,
    new_manifest: Vec<u8>,
}

struct AcceptanceLock {
    file: File,
}

impl Drop for AcceptanceLock {
    fn drop(&mut self) {
        let _ = self.file.unlock();
    }
}

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
        self.recover_acceptance()?;
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
        self.recover_acceptance()?;
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
        self.recover_acceptance()?;
        self.validate_candidate(identity)
    }

    fn validate_candidate(&self, identity: CandidateIdentity) -> Result<CandidateIdentity, String> {
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
        self.recover_acceptance()?;
        let identity = self.begin_candidate(identity)?;
        let supersedes = match self.candidate_state(&identity.artifact_id)? {
            None => None,
            Some(("rejected", revision)) => Some(revision),
            Some(_) => {
                return Err(
                    "a staged candidate already exists; replacement is not supported".into(),
                )
            }
        };
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
        let bytes = format!("{frontmatter}\n{body}").into_bytes();
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
        if supersedes.as_deref() == Some(revision.as_str()) {
            return Err("replacement candidate must have a new revision".into());
        }
        let staged = StagedCandidate {
            identity,
            bytes,
            content,
            revision,
            supersedes,
            state: "staged".into(),
        };
        self.prepare_staged_dir()?;
        let staged_path = self.staged_path(&staged.identity.artifact_id)?;
        let serialized = serde_json::to_vec(&staged).map_err(|error| error.to_string())?;
        if staged.supersedes.is_some() {
            replace_staged_json(&staged_path, &serialized)?;
        } else {
            write_new_json(
                &staged_path,
                &staged,
                "a staged candidate already exists; replacement is not supported",
            )?;
        }
        Ok(staged)
    }

    pub fn read_staged_candidate(&self, artifact_id: &str) -> Result<StagedCandidate, String> {
        self.recover_acceptance()?;
        self.validated_staged_candidate(artifact_id)
    }

    pub fn begin_candidate_review(
        &self,
        artifact_id: &str,
        candidate_revision: &str,
    ) -> Result<CandidateReviewRequest, String> {
        self.recover_acceptance()?;
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
        self.recover_acceptance()?;
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

    pub fn accept_candidate(
        &self,
        artifact_id: &str,
        candidate_revision: &str,
    ) -> Result<AcceptedRevision, String> {
        let _lock = self.acquire_acceptance_lock()?;
        self.recover_acceptance_locked()?;
        let candidate = self.matching_staged_candidate(artifact_id, candidate_revision)?;
        match self.candidate_state(artifact_id)? {
            Some(("approved", revision)) if revision == candidate.revision => {}
            _ => return Err("candidate is not approved".into()),
        }
        self.validate_candidate(candidate.identity.clone())?;

        let mut manifest = self.manifest()?;
        let descriptor = manifest
            .artifacts
            .get_mut(artifact_id)
            .ok_or_else(|| "unknown artifact".to_owned())?;
        let artifact_relative_path = descriptor.representation.path.clone();
        let artifact_path = self.artifact_path(&artifact_relative_path)?;
        let old_artifact = if artifact_path.exists() {
            Some(fs::read(&artifact_path).map_err(|error| error.to_string())?)
        } else {
            None
        };
        if let Some(accepted) = &descriptor.accepted {
            verify_content(
                old_artifact
                    .as_deref()
                    .ok_or("accepted artifact is missing")?,
                &accepted.content,
            )?;
        }
        verify_content(&candidate.bytes, &candidate.content)?;
        let accepted = AcceptedRevision {
            revision: candidate.revision.clone(),
            content: candidate.content.clone(),
            sources: candidate.identity.source_revisions.clone(),
        };
        descriptor.accepted = Some(accepted.clone());
        let old_manifest = fs::read(&self.manifest_path).map_err(|error| error.to_string())?;
        let new_manifest = serde_yaml::to_string(&manifest)
            .map_err(|error| error.to_string())?
            .into_bytes();
        let journal = AcceptanceJournal {
            artifact_id: artifact_id.into(),
            candidate_revision: candidate.revision,
            artifact_path: artifact_relative_path,
            old_artifact,
            old_manifest,
            new_manifest: new_manifest.clone(),
        };
        self.prepare_recovery_dir()?;
        let journal_path = self.acceptance_journal_path(artifact_id)?;
        write_new_json(
            &journal_path,
            &journal,
            "acceptance recovery already exists",
        )?;
        let artifact_temp = temporary_path(&artifact_path, "accept");
        let manifest_temp = temporary_path(&self.manifest_path, "accept");
        let staged_path = self.staged_path(artifact_id)?;
        if let Err(error) = write_temporary_file(&artifact_temp, &candidate.bytes)
            .and_then(|_| write_temporary_file(&manifest_temp, &new_manifest))
            .and_then(|_| {
                fs::rename(&artifact_temp, &artifact_path).map_err(|error| error.to_string())
            })
            .and_then(|_| {
                fs::rename(&manifest_temp, &self.manifest_path).map_err(|error| error.to_string())
            })
        {
            let _ = fs::remove_file(&artifact_temp);
            let _ = fs::remove_file(&manifest_temp);
            self.recover_acceptance_locked()?;
            return Err(error);
        }
        Ok(self.complete_acceptance_after_manifest(accepted, &staged_path, artifact_id))
    }

    fn manifest(&self) -> Result<Manifest, String> {
        serde_yaml::from_slice(&fs::read(&self.manifest_path).map_err(|error| error.to_string())?)
            .map_err(|error| error.to_string())
    }
    fn recovery_dir(&self) -> Result<PathBuf, String> {
        let root = fs::canonicalize(&self.root).map_err(|error| error.to_string())?;
        let rmwm = validate_directory(&root, &root.join(".rmwm"), "rmwm directory")?;
        Ok(rmwm.join("recovery"))
    }
    fn acquire_acceptance_lock(&self) -> Result<AcceptanceLock, String> {
        self.prepare_staged_dir()?;
        let lock_path = self
            .root
            .join(".rmwm")
            .join("requirements-model.accept.lock");
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(false)
            .open(&lock_path)
            .map_err(|error| error.to_string())?;
        file.try_lock_exclusive().map_err(|error| {
            if error.kind() == std::io::ErrorKind::WouldBlock {
                "requirements model is locked".to_owned()
            } else {
                error.to_string()
            }
        })?;
        Ok(AcceptanceLock { file })
    }
    fn prepare_recovery_dir(&self) -> Result<(), String> {
        self.prepare_staged_dir()?;
        let recovery = self.recovery_dir()?;
        if recovery.exists() {
            let root = fs::canonicalize(&self.root).map_err(|error| error.to_string())?;
            validate_directory(&root, &recovery, "recovery directory")?;
        } else {
            fs::create_dir(&recovery).map_err(|error| error.to_string())?;
        }
        Ok(())
    }
    fn acceptance_journal_path(&self, artifact_id: &str) -> Result<PathBuf, String> {
        validate_artifact_id(artifact_id)?;
        Ok(self
            .recovery_dir()?
            .join(format!("{artifact_id}.accept.json")))
    }
    fn clear_acceptance_recovery(&self, artifact_id: &str) -> Result<(), String> {
        let journal = self.acceptance_journal_path(artifact_id)?;
        if journal.exists() {
            fs::remove_file(journal).map_err(|error| error.to_string())?;
        }
        Ok(())
    }
    fn complete_acceptance_after_manifest(
        &self,
        accepted: AcceptedRevision,
        staged_path: &Path,
        artifact_id: &str,
    ) -> AcceptedRevision {
        if fs::remove_file(staged_path).is_ok() {
            let _ = self.clear_acceptance_recovery(artifact_id);
        }
        accepted
    }
    fn recover_acceptance(&self) -> Result<(), String> {
        if !self.root.join(".rmwm").exists() {
            return Ok(());
        }
        let _lock = self.acquire_acceptance_lock()?;
        self.recover_acceptance_locked()
    }
    fn recover_acceptance_locked(&self) -> Result<(), String> {
        if !self.root.join(".rmwm").exists() {
            return Ok(());
        }
        let recovery = self.recovery_dir()?;
        if !recovery.exists() {
            return Ok(());
        }
        for entry in fs::read_dir(recovery).map_err(|error| error.to_string())? {
            let path = entry.map_err(|error| error.to_string())?.path();
            let journal: AcceptanceJournal = read_json(&path, "invalid acceptance recovery")?;
            let artifact_path = self.artifact_path(&journal.artifact_path)?;
            let staged_path = self.staged_path(&journal.artifact_id)?;
            let manifest = fs::read(&self.manifest_path).map_err(|error| error.to_string())?;
            if manifest == journal.old_manifest {
                match journal.old_artifact {
                    Some(bytes) => {
                        fs::write(&artifact_path, bytes).map_err(|error| error.to_string())?
                    }
                    None if artifact_path.exists() => {
                        fs::remove_file(&artifact_path).map_err(|error| error.to_string())?
                    }
                    None => {}
                }
            } else if manifest == journal.new_manifest {
                let current = self.manifest()?;
                let descriptor = current
                    .artifacts
                    .get(&journal.artifact_id)
                    .ok_or("invalid acceptance recovery manifest")?;
                let accepted = descriptor
                    .accepted
                    .as_ref()
                    .ok_or("invalid acceptance recovery manifest")?;
                if accepted.revision != journal.candidate_revision {
                    return Err("invalid acceptance recovery manifest".into());
                }
                verify_content(
                    &fs::read(&artifact_path).map_err(|error| error.to_string())?,
                    &accepted.content,
                )?;
                if staged_path.exists() {
                    fs::remove_file(&staged_path).map_err(|error| error.to_string())?;
                }
            } else {
                return Err("unresolved acceptance recovery".into());
            }
            remove_file_if_exists(&temporary_path(&artifact_path, "accept"))?;
            remove_file_if_exists(&temporary_path(&self.manifest_path, "accept"))?;
            fs::remove_file(path).map_err(|error| error.to_string())?;
        }
        Ok(())
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
        self.validate_candidate(candidate.identity.clone())?;
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
    file.write_all(&bytes)
        .map_err(|error| error.to_string())
        .and_then(|_| file.sync_all().map_err(|error| error.to_string()))
}

fn replace_staged_json(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let temporary_path = path.with_extension("json.tmp");
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary_path)
        .map_err(|error| error.to_string())?;
    let result = (|| {
        file.write_all(bytes).map_err(|error| error.to_string())?;
        file.sync_all().map_err(|error| error.to_string())?;
        drop(file);
        fs::rename(&temporary_path, path).map_err(|error| error.to_string())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary_path);
    }
    result
}

fn temporary_path(path: &Path, suffix: &str) -> PathBuf {
    path.with_extension(format!(
        "{}.{}",
        path.extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or_default(),
        suffix
    ))
}

fn write_temporary_file(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| error.to_string())?;
    file.write_all(bytes).map_err(|error| error.to_string())?;
    file.sync_all().map_err(|error| error.to_string())
}

fn remove_file_if_exists(path: &Path) -> Result<(), String> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn failed_staged_replacement_removes_its_temporary_file() {
        let root = std::env::temp_dir().join(format!(
            "rmwm-replace-staged-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        let staged_path = root.join("candidate.json");
        fs::create_dir(&staged_path).unwrap();

        assert!(replace_staged_json(&staged_path, b"replacement").is_err());
        assert!(!staged_path.with_extension("json.tmp").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn post_manifest_cleanup_failure_does_not_revoke_acceptance() {
        let root = std::env::temp_dir().join(format!(
            "rmwm-accept-cleanup-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(root.join(".rmwm/recovery")).unwrap();
        let staged_path = root.join(".rmwm/staged/candidate.json");
        fs::create_dir_all(&staged_path).unwrap();
        let store = ModelStore::open(&root);
        let artifact = root.join("domain_framing.md");
        fs::write(&artifact, b"accepted").unwrap();
        let accepted = AcceptedRevision {
            revision: "sha256:accepted".into(),
            content: ContentDescriptor {
                digest: content_digest(b"accepted"),
                size: b"accepted".len(),
            },
            sources: BTreeMap::new(),
        };
        let manifest = format!(
            "schema: rmwm/requirements-model/v1\nmodel_id: test\nartifacts:\n  candidate:\n    type: domain_framing\n    representation:\n      path: domain_framing.md\n      media_type: text/markdown\n      encoding: utf-8\n      line_endings: lf\n    accepted:\n      revision: {}\n      content:\n        digest: {}\n        size: {}\n      sources: {{}}\n",
            accepted.revision, accepted.content.digest, accepted.content.size
        )
        .into_bytes();
        fs::write(root.join("requirements_model.yaml"), &manifest).unwrap();
        let journal_path = root.join(".rmwm/recovery/candidate.accept.json");
        let journal = AcceptanceJournal {
            artifact_id: "candidate".into(),
            candidate_revision: accepted.revision.clone(),
            artifact_path: "domain_framing.md".into(),
            old_artifact: None,
            old_manifest: b"old manifest".to_vec(),
            new_manifest: manifest,
        };
        write_new_json(&journal_path, &journal, "journal exists").unwrap();
        let artifact_temp = temporary_path(&artifact, "accept");
        let manifest_temp = temporary_path(&root.join("requirements_model.yaml"), "accept");
        fs::write(&artifact_temp, b"interrupted artifact write").unwrap();
        fs::write(&manifest_temp, b"interrupted manifest write").unwrap();

        let result = store.complete_acceptance_after_manifest(
            accepted.clone(),
            &staged_path,
            "raw-adc-domain-framing",
        );

        assert_eq!(result.revision, accepted.revision);
        assert!(staged_path.exists());
        assert!(journal_path.exists());
        fs::remove_dir(&staged_path).unwrap();
        fs::write(&staged_path, b"stale candidate").unwrap();
        store.recover_acceptance().unwrap();
        assert!(!staged_path.exists());
        assert!(!artifact_temp.exists());
        assert!(!manifest_temp.exists());
        assert!(!journal_path.exists());
        write_temporary_file(&artifact_temp, b"next artifact write").unwrap();
        write_temporary_file(&manifest_temp, b"next manifest write").unwrap();
        assert!(artifact_temp.exists());
        assert!(manifest_temp.exists());
        fs::remove_dir_all(root).unwrap();
    }
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
