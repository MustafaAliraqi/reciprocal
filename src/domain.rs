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
    Autoregressive, Diffusion, ArThenDiffusion, DiffusionThenAr, Parallel, Hierarchical,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BlackboardKind { Plan, Observation, Artifact, Decision, Review, Output }

impl BlackboardKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Plan => "plan",
            Self::Observation => "observation",
            Self::Artifact => "artifact",
            Self::Decision => "decision",
            Self::Review => "review",
            Self::Output => "output",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct BlackboardEntry {
    pub task_id: String,
    pub sequence: u64,
    pub key: String,
    pub kind: BlackboardKind,
    pub architecture: Architecture,
    pub provider: String,
    pub content: String,
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
    #[serde(default)]
    pub supports_tools: bool,
}

#[derive(Debug, Deserialize)]
pub struct NewProvider {
    pub name: String,
    pub architecture: Architecture,
    pub adapter: AdapterKind,
    pub endpoint: Option<String>,
    pub model: String,
    pub api_key: Option<String>,
    #[serde(default)]
    pub supports_tools: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolEvent {
    pub name: String,
    pub arguments: serde_json::Value,
    pub output: String,
    pub succeeded: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct TaskResult {
    pub id: Uuid,
    pub workflow: Workflow,
    pub output: String,
    pub stages: Vec<StageResult>,
    pub tool_events: Vec<ToolEvent>,
    pub blackboard: Vec<BlackboardEntry>,
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
    #[serde(default)]
    pub allow_workspace_writes: bool,
}

#[cfg(test)]
mod compatibility_tests {
    use super::*;

    #[test]
    fn old_provider_config_defaults_tool_capability_to_disabled() {
        let value = serde_json::json!({
            "id":"legacy", "name":"Legacy", "architecture":"autoregressive",
            "adapter":"open_ai_compatible", "endpoint":"https://example.invalid/v1",
            "model":"model", "secret_id":"legacy"
        });
        let provider: ProviderConfig = serde_json::from_value(value).unwrap();
        assert!(!provider.supports_tools);
    }

    #[test]
    fn old_task_request_defaults_workspace_writes_to_disabled() {
        let value = serde_json::json!({
            "task":"inspect project", "workflow":"autoregressive",
            "autoregressive_provider":"p"
        });
        let request: TaskRequest = serde_json::from_value(value).unwrap();
        assert!(!request.allow_workspace_writes);
    }
}
