use axum::serve;
use collector_core::{app, AppState};
use std::net::SocketAddr;

#[tokio::main]
async fn main() {
    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(3000);
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("bind listener");
    println!("collector listening on http://{}", addr);
    serve(listener, app(AppState::new()))
        .await
        .expect("server failed");
}
