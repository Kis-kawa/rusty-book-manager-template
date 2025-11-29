use axum::{
    routing::{get, post},
    Router,
    Json,
    extract::State,
    http::{Method, StatusCode},
};
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tower_http::cors::{CorsLayer, Any};
use serde::Deserialize;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use bcrypt::{hash, verify, DEFAULT_COST};

#[tokio::main]
async fn main() {
    // 環境変数を読み込む
    dotenv::dotenv().ok();
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    // DB接続プールを作成
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("can't connect to database");

    println!("Database connected successfully!");

    // CORS設定
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(vec![Method::GET, Method::POST])
        .allow_headers(Any);

    // ルーティング
    // ここで .with_state(pool) をしているため、
    // 全てのハンドラ（関数）は State<PgPool> を受け取る形か、
    // 全くStateを使わない形のどちらかである必要があります。
    let app = Router::new()
        .route("/", get(|| async { "Hello from DB Connected Server!" }))
        .route("/login", post(login_handler))
        .route("/register", post(register_handler))
        .layer(cors)
        .with_state(pool);

    // サーバー起動
    let addr = SocketAddr::from(([0, 0, 0, 0], 8000));
    println!("Server listening on {}", addr);

    let listener = TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

// ----------------------------------------------------------------
// 型定義 (Structs)
// ----------------------------------------------------------------

#[derive(Deserialize)]
struct LoginRequest {
    email: String,
    password: String,
}

#[derive(Deserialize)]
struct RegisterRequest {
    name: String,
    email: String,
    password: String,
    role: String,
}

// ----------------------------------------------------------------
// ハンドラ関数 (Handlers)
// ----------------------------------------------------------------

async fn login_handler(
    State(pool): State<PgPool>,
    Json(payload): Json<LoginRequest>
) -> Result<String, StatusCode> {
    println!("【ログイン】リクエスト受信: {}", payload.email);

    // A. データベースからユーザーを探す
    // fetch_optional は「見つかったら Some(user), 見つからなかったら None」を返します
    let user = sqlx::query!(
        "SELECT user_id, name, password FROM users WHERE email = $1",
        payload.email
    )
    .fetch_optional(&pool)
    .await
    .map_err(|e| {
        println!("❌ DBエラー: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // B. ユーザーが存在するかチェック
    let user = match user {
        Some(u) => u,
        None => {
            println!("❌ ユーザーが見つかりません: {}", payload.email);
            return Err(StatusCode::UNAUTHORIZED); // 401 Unauthorized
        }
    };

    // C. パスワードが合っているかチェック (verify)
    // payload.password (入力された平文) と user.password (DBのハッシュ) を比較
    let is_valid = verify(payload.password, &user.password)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if is_valid {
        println!("✅ ログイン成功: {}", user.name);
        // 本来はここで「セッショントークン」などを返しますが、今は成功メッセージだけでOK
        Ok(format!("Login successful! Welcome, {} (ID: {})", user.name, user.user_id))
    } else {
        println!("❌ パスワード不一致: {}", payload.email);
        Err(StatusCode::UNAUTHORIZED) // 401 Unauthorized
    }
}


async fn register_handler(
    State(pool): State<PgPool>,
    Json(payload): Json<RegisterRequest>,
) -> Result<String, StatusCode> {
    println!("【登録】リクエスト受信: {}", payload.email);

    // パスワードのハッシュ化
    let hashed_password = hash(payload.password, DEFAULT_COST)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // DBへの保存
    let result = sqlx::query!(
        r#"
        INSERT INTO users (name, email, password, role)
        VALUES ($1, $2, $3, $4::text::user_role)
        RETURNING user_id
        "#,
        payload.name,
        payload.email,
        hashed_password,
        payload.role
    )
    .fetch_one(&pool)
    .await;

    match result {
        Ok(record) => {
            println!("ユーザー登録成功! ID: {}", record.user_id);
            Ok(format!("User created with ID: {}", record.user_id))
        }
        Err(e) => {
            println!("データベースエラー: {:?}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
