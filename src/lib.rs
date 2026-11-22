pub mod domain;
pub mod orchestrator;
pub mod providers;
pub mod workspace;

use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, StatusCode, Uri},
    response::Response,
    routing::get,
    Json, Router,
};
use domain::{NewProvider, TaskRequest};
use providers::{AppState, ProviderView};
use rust_embed::RustEmbed;
use std::sync::Arc;

#[derive(RustEmbed)]
#[folder = "static/"]
struct WebAssets;

pub fn app() -> Router {
    let state = Arc::new(AppState::load());
    Router::new()
        .route("/api/health", get(|| async { Json(serde_json::json!({"status":"ready"})) }))
        .route("/api/providers", get(list_providers).post(add_provider))
        .route("/api/providers/:id", axum::routing::delete(remove_provider))
        .route("/api/tasks", axum::routing::post(run_task))
        .route("/api/workspace", get(workspace::get_workspace).post(workspace::set_workspace))
        .route("/api/workspace/files", get(workspace::list_files))
        .route("/api/workspace/read", get(workspace::read_file))
        .route("/api/workspace/write", axum::routing::post(workspace::write_file))
        .route("/api/workspace/search", get(workspace::search_files))
        .with_state(state)
        .fallback(static_file)
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
    Path(id): Path<String>,
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

async fn static_file(uri: Uri) -> Response<Body> {
    let requested = uri.path().trim_start_matches('/');
    let path = if requested.is_empty() { "index.html" } else { requested };
    match WebAssets::get(path) {
        Some(asset) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, mime.as_ref())
                .body(Body::from(asset.data.into_owned()))
                .expect("valid static response")
        }
        None => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("Not found"))
            .expect("valid 404 response"),
    }
}
