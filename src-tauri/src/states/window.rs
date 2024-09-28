use std::path::PathBuf;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct OpenFilePayload {
    pub path: PathBuf,
}
