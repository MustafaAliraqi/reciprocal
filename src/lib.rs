pub mod domain;
pub mod orchestrator;
pub mod providers;

use axum::{extract::State, http::StatusCode, routing::{get, post}, Json, Router};
use domain::{NewProvider, TaskRequest};
use providers::{AppState, ProviderView};
use std::sync::Arc;

pub fn app() -> Router {
    let state = Arc::new(AppState::load());
    Router::new()
        .route("/api/health", get(|| async { Json(serde_json::json!({"status":"ready"})) }))
        .route("/api/providers", get(list_providers).post(add_provider))
        .route("/api/providers/:id", axum::routing::delete(remove_provider))
        .route("/api/tasks", post(run_task))
        .with_state(state)
        .fallback_service(tower_http::services::ServeDir::new("static"))
}

async fn list_providers(State(state): State<Arc<AppState>>) -> Json<Vec<ProviderView>> {
    Json(state.provider_views())
}

async fn add_provider(
    State(state): State<Arc<AppState>>,
    Json(input): Json<NewProvider>,
) -> Result<(StatusCode, Json<ProviderView>), (StatusCode, String)> {
    let provider = state.add_provider(input).map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    Ok((StatusCode::CREATED, Json(provider)))
}

async fn remove_provider(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    state.remove_provider(&id).map_err(|e| (StatusCode::NOT_FOUND, e.to_string()))?;
    Ok(StatusCode::NO_CONTENT)
}

async fn run_task(
    State(state): State<Arc<AppState>>,
    Json(request): Json<TaskRequest>,
) -> Result<Json<domain::TaskResult>, (StatusCode, String)> {
    if request.task.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "اكتب المهمة أولاً".into()));
    }
    orchestrator::execute(state, request).await
        .map(Json)
        .map_err(|e| (StatusCode::BAD_GATEWAY, e.to_string()))
}
