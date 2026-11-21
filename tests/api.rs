use axum::{body::Body, http::{Request, StatusCode}};
use http_body_util::BodyExt;
use reciprocal::app;
use tower::ServiceExt;

#[tokio::test]
async fn health_endpoint_returns_ok() {
    let response = app().oneshot(Request::builder().uri("/api/health").body(Body::empty()).unwrap()).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn rejects_empty_task() {
    let response = app().oneshot(
        Request::builder().method("POST").uri("/api/runs")
            .header("content-type","application/json")
            .body(Body::from(r#"{"task":" ","strategy":"parallel","repetitions":1}"#)).unwrap()
    ).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn creates_simulated_run() {
    let response = app().oneshot(
        Request::builder().method("POST").uri("/api/runs")
            .header("content-type","application/json")
            .body(Body::from(r#"{"task":"test task","strategy":"ar_only","repetitions":1}"#)).unwrap()
    ).await.unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body = String::from_utf8(bytes.to_vec()).unwrap();
    assert!(body.contains("\"simulated\""));
}
