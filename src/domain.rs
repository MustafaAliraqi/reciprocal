use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Architecture { Autoregressive, Diffusion }

impl Architecture {
    pub fn label(self) -> &'static str {
        match self { Self::Autoregressive => "Autoregressive", Self::Diffusion => "Language diffusion" }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AdapterKind { OpenAiCompatible, CodexCli }

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Workflow {
    Autoregressive,
    Diffusion,
    ArThenDiffusion,
    DiffusionThenAr,
    Parallel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub id: String,
    pub name: String,
    pub architecture: Architecture,
    pub adapter: AdapterKind,
    pub endpoint: String,
    pub model: String,
    pub secret_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct NewProvider {
    pub name: String,
    pub architecture: Architecture,
    pub adapter: AdapterKind,
    pub endpoint: Option<String>,
    pub model: String,
    pub api_key: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TaskResult {
    pub id: Uuid,
    pub workflow: Workflow,
    pub output: String,
    pub stages: Vec<StageResult>,
}

#[derive(Debug, Clone, Serialize)]
pub struct StageResult {
    pub architecture: Architecture,
    pub provider: String,
    pub output: String,
}

#[derive(Debug, Deserialize)]
pub struct TaskRequest {
    pub task: String,
    pub workflow: Workflow,
    pub autoregressive_provider: Option<String>,
    pub diffusion_provider: Option<String>,
}
