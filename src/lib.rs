pub mod domain;
pub mod orchestrator;

use axum::{extract::State, http::StatusCode, response::IntoResponse, routing::get, Json, Router};
use domain::{ExperimentRequest, ExperimentResult, Strategy};
use orchestrator::run_experiment;
use std::sync::{Arc, Mutex};

#[derive(Clone, Default)]
struct AppState {
    runs: Arc<Mutex<Vec<ExperimentResult>>>,
}

pub fn app() -> Router {
    let state = AppState::default();
    Router::new()
        .route("/api/health", get(health))
        .route("/api/strategies", get(strategies))
        .route("/api/runs", get(list_runs).post(create_run))
        .with_state(state)
        .fallback_service(tower_http::services::ServeDir::new("static"))
}

async fn health() -> impl IntoResponse {
    Json(serde_json::json!({"status":"ok","project":"reciprocal"}))
}

async fn strategies() -> Json<Vec<Strategy>> {
    Json(Strategy::all())
}

async fn list_runs(State(state): State<AppState>) -> Json<Vec<ExperimentResult>> {
    let runs = state.runs.lock().expect("runs lock");
    Json(runs.clone())
}

async fn create_run(
    State(state): State<AppState>,
    Json(request): Json<ExperimentRequest>,
) -> Result<(StatusCode, Json<ExperimentResult>), (StatusCode, String)> {
    if request.task.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "task must not be empty".into()));
    }
    if !(1..=20).contains(&request.repetitions) {
        return Err((StatusCode::BAD_REQUEST, "repetitions must be between 1 and 20".into()));
    }
    let result = run_experiment(request);
    state.runs.lock().map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "storage lock poisoned".into()))?.push(result.clone());
    Ok((StatusCode::CREATED, Json(result)))
}
