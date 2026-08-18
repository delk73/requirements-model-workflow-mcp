use sha2::{Digest, Sha256};

pub fn content_digest(bytes: &[u8]) -> String {
    format!("sha256:{}", hex(&Sha256::digest(bytes)))
}

pub fn revision_handle(
    model_id: &str,
    artifact_id: &str,
    artifact_type: &str,
    stage: &str,
    content_digest: &str,
    sources: &[(String, String)],
) -> String {
    let mut bytes = Vec::new();
    for value in [
        "rmwm-revision-v1",
        model_id,
        artifact_id,
        artifact_type,
        stage,
        content_digest,
    ] {
        append_string(&mut bytes, value);
    }
    let mut sources = sources.to_vec();
    sources.sort_by(|left, right| left.0.cmp(&right.0));
    bytes.extend_from_slice(&(sources.len() as u64).to_be_bytes());
    for (artifact_id, revision) in sources {
        append_string(&mut bytes, &artifact_id);
        append_string(&mut bytes, &revision);
    }
    format!("sha256:{}", hex(&Sha256::digest(bytes)))
}

fn append_string(bytes: &mut Vec<u8>, value: &str) {
    bytes.extend_from_slice(&(value.len() as u64).to_be_bytes());
    bytes.extend_from_slice(value.as_bytes());
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
