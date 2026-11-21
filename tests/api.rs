use axum::{body::Body, http::{Request, StatusCode}};
use http_body_util::BodyExt;
use reciprocal::app;
use tower::ServiceExt;

#[tokio::test]
async fn health_endpoint_returns_ready() {
    let response = app().oneshot(Request::builder().uri("/api/health").body(Body::empty()).unwrap()).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn provider_list_returns_json_array() {
    let response = app().oneshot(Request::builder().uri("/api/providers").body(Body::empty()).unwrap()).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let parsed: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(parsed.is_array());
}

#[tokio::test]
async fn rejects_empty_task() {
    let response = app().oneshot(
        Request::builder().method("POST").uri("/api/tasks")
            .header("content-type","application/json")
            .body(Body::from(r#"{"task":" ","workflow":"parallel"}"#)).unwrap()
    ).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}
