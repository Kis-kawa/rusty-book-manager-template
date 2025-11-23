use axum::{
    routing::{get, post},
    Router,
    Json,
    http::Method,
};
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tower_http::cors::{CorsLayer, Any};
use serde::Deserialize;

#[tokio::main]
async fn main() {
    // 1. CORS設定（通行手形の発行）
    // Frontend (localhost:3000) からのアクセスを許可します
    let cors = CorsLayer::new()
        .allow_origin(Any) // 本番では "http://localhost:3000".parse().unwrap() のように厳密にします
        .allow_methods(vec![Method::GET, Method::POST]) // GETとPOSTを許可
        .allow_headers(Any);

    let app = Router::new()
        .route("/", get(|| async { "Hello from Textbook Structure!" }))
        .route("/login", post(login_handler)) // POST /login が来たら login_handler を動かす
        .layer(cors); // CORSを適用

    let addr = SocketAddr::from(([0, 0, 0, 0], 8000));
    println!("Server listening on {}", addr);

    let listener = TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

// ----------------------------------------------------------------
// データを受け取るための構造体（型定義）
// ----------------------------------------------------------------
#[derive(Deserialize)]
struct LoginRequest {
    email: String,
    // password: String, // 今回はログに出すだけなので使わないけど、本来は必要
}

// ----------------------------------------------------------------
// ログイン処理（ハンドラ）
// ----------------------------------------------------------------
// Frontendから送られてきたJSONを LoginRequest 型に自動変換して受け取る
async fn login_handler(Json(payload): Json<LoginRequest>) -> String {
    println!("【受信】ログインリクエスト: {}", payload.email);

    // Frontendに返事を返す
    format!("Rust backend received: {}", payload.email)
}
