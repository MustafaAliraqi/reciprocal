use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Strategy {
    ArOnly,
    DiffusionOnly,
    ArThenDiffusion,
    DiffusionThenAr,
    Parallel,
    Hierarchical,
}

impl Strategy {
    pub fn all() -> Vec<Self> {
        vec![Self::ArOnly, Self::DiffusionOnly, Self::ArThenDiffusion, Self::DiffusionThenAr, Self::Parallel, Self::Hierarchical]
    }
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ArOnly => "ar_only",
            Self::DiffusionOnly => "diffusion_only",
            Self::ArThenDiffusion => "ar_then_diffusion",
            Self::DiffusionThenAr => "diffusion_then_ar",
            Self::Parallel => "parallel",
            Self::Hierarchical => "hierarchical",
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ExperimentRequest {
    pub task: String,
    pub strategy: Strategy,
    #[serde(default = "default_repetitions")]
    pub repetitions: u8,
}
fn default_repetitions() -> u8 { 1 }

#[derive(Debug, Clone, Serialize)]
pub struct ExperimentResult {
    pub id: Uuid,
    pub task: String,
    pub strategy: Strategy,
    pub repetitions: u8,
    pub quality: f64,
    pub latency_ms: u64,
    pub estimated_cost_units: f64,
    pub correction_cycles: u32,
    pub status: &'static str,
    pub note: &'static str,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exposes_all_six_strategies() {
        assert_eq!(Strategy::all().len(), 6);
        assert_eq!(Strategy::Parallel.as_str(), "parallel");
    }
}
