use reciprocal::app;

#[tokio::main]
async fn main() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await.expect("bind local server");
    let address = listener.local_addr().expect("read local address");
    let url = format!("http://{address}");
    println!("Reciprocal is ready at {url}");
    if let Err(error) = webbrowser::open(&url) {
        eprintln!("Could not open the browser automatically: {error}");
        println!("Open this address manually: {url}");
    }
    axum::serve(listener, app()).await.expect("serve Reciprocal");
}
