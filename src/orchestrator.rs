use crate::domain::{ExperimentRequest, ExperimentResult, Strategy};
use uuid::Uuid;

/// Deterministic placeholder simulator. Replace this with measured provider calls
/// before drawing conclusions about model quality, cost, or latency.
pub fn run_experiment(request: ExperimentRequest) -> ExperimentResult {
    let (quality, latency, cost, cycles) = match request.strategy {
        Strategy::ArOnly => (0.62, 420, 1.0, 0),
        Strategy::DiffusionOnly => (0.60, 510, 1.1, 0),
        Strategy::ArThenDiffusion => (0.68, 790, 2.0, 1),
        Strategy::DiffusionThenAr => (0.67, 810, 2.1, 1),
        Strategy::Parallel => (0.69, 560, 2.1, 0),
        Strategy::Hierarchical => (0.72, 980, 3.0, 2),
    };
    ExperimentResult {
        id: Uuid::new_v4(),
        task: request.task,
        strategy: request.strategy,
        repetitions: request.repetitions,
        quality,
        latency_ms: latency * u64::from(request.repetitions),
        estimated_cost_units: cost * f64::from(request.repetitions),
        correction_cycles: cycles * u32::from(request.repetitions),
        status: "simulated",
        note: "Illustrative simulator output; not a measurement from an AI model.",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn output_is_explicitly_marked_simulated() {
        let result = run_experiment(ExperimentRequest {
            task: "test".into(), strategy: Strategy::Parallel, repetitions: 1
        });
        assert_eq!(result.status, "simulated");
        assert!(result.latency_ms > 0);
    }
}
