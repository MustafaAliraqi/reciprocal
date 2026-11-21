use reciprocal::app;

#[tokio::main]
async fn main() {
    let app = app();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .expect("bind 127.0.0.1:3000");
    println!("Reciprocal listening at http://{}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.expect("server");
}
