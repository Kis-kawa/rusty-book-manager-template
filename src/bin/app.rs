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
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use bcrypt::{hash, verify, DEFAULT_COST};
use chrono::NaiveDateTime;

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
        .route("/trips", get(get_all_trips))
        .route("/reservations", post(create_reservation))
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

#[derive(Serialize)]
struct LoginResponse {
    user_id: uuid::Uuid,
    name: String,
}

#[derive(Serialize)]
struct TripResponse {
    trip_id: uuid::Uuid,
    source: String,      // 出発地名
    destination: String, // 到着地名
    departure_time: NaiveDateTime, // 出発日時
    arrival_time: NaiveDateTime,   // 到着日時
    vehicle_name: String, // 車両名 (産技号1など)
    status: String,       // 運行状況 (scheduled, delayed...)
}

#[derive(Deserialize)]
struct CreateReservationRequest {
    trip_id: uuid::Uuid,
    user_id: uuid::Uuid,
}

// ----------------------------------------------------------------
// ハンドラ関数 (Handlers)
// ----------------------------------------------------------------

// login
async fn login_handler(
    State(pool): State<PgPool>,
    Json(payload): Json<LoginRequest>
) -> Result<Json<LoginResponse>, StatusCode> {
    println!("【ログイン】リクエスト受信: {}", payload.email);

    // データベースからユーザーを探す
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

    // ユーザーが存在するかチェック
    let user = match user {
        Some(u) => u,
        None => {
            println!("❌ ユーザーが見つかりません: {}", payload.email);
            return Err(StatusCode::UNAUTHORIZED); // 401 Unauthorized
        }
    };

    // パスワードが合っているかチェック (verify)
    // payload.password (入力された平文) と user.password (DBのハッシュ) を比較
    let is_valid = verify(payload.password, &user.password)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if is_valid {
        println!("✅ ログイン成功: {}", user.name);

        let response = LoginResponse {
            user_id: user.user_id,
            name: user.name,
        };
        Ok(Json(response))
    } else {
        println!("❌ パスワード不一致: {}", payload.email);
        Err(StatusCode::UNAUTHORIZED)
    }
}


//singup
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


// 運行便の一覧
async fn get_all_trips(
    State(pool): State<PgPool>
) -> Result<Json<Vec<TripResponse>>, StatusCode> {

    // 複数のテーブルを結合(JOIN)して、必要な情報を一度に取ってくるSQL
    // COALESCE(os.status::text, 'scheduled')
    // → operational_statuses にレコードがあればそれを使い、なければ 'scheduled' (平常) とする
    let rows = sqlx::query!(
        r#"
        SELECT
            t.trip_id,
            t.departure_datetime,
            t.arrival_datetime,
            s_stop.name as "source_name!",    -- !をつけると「NULLにならない」とRustに教えられる
            d_stop.name as "dest_name!",
            v.vehicle_name as "vehicle_name!",
            COALESCE(os.status::text, 'scheduled') as "status!"
        FROM trips t
        JOIN routes r ON t.route_id = r.route_id
        JOIN bus_stops s_stop ON r.source_bus_stop_id = s_stop.bus_stop_id
        JOIN bus_stops d_stop ON r.destination_bus_stop_id = d_stop.bus_stop_id
        JOIN vehicles v ON t.vehicle_id = v.vehicle_id
        LEFT JOIN operational_statuses os ON t.trip_id = os.trip_id
        ORDER BY t.departure_datetime ASC
        "#
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| {
        println!("❌ DBエラー: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // DBから取れたデータを、レスポンス用の型に詰め替える
    let trips = rows.into_iter().map(|row| TripResponse {
        trip_id: row.trip_id,
        source: row.source_name,
        destination: row.dest_name,
        departure_time: row.departure_datetime,
        arrival_time: row.arrival_datetime,
        vehicle_name: row.vehicle_name,
        status: row.status,
    }).collect();

    Ok(Json(trips))
}


// 予約作成 (POST /reservations)
async fn create_reservation(
    State(pool): State<PgPool>,
    Json(payload): Json<CreateReservationRequest>,
) -> Result<String, StatusCode> {
    println!("【予約】Trip: {}, User: {}", payload.trip_id, payload.user_id);

    // trips -> vehicles -> vehicle_types と辿って total_seats、車両の定員を取ってくる
    let capacity = sqlx::query!(
        r#"
        SELECT vt.total_seats
        FROM trips t
        JOIN vehicles v ON t.vehicle_id = v.vehicle_id
        JOIN vehicle_types vt ON v.vehicle_type_id = vt.vehicle_type_id
        WHERE t.trip_id = $1
        "#,
        payload.trip_id
    )
    .fetch_one(&pool)
    .await
    .map_err(|e| {
        println!("DBエラー(定員取得): {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?
    .total_seats;

    // 次の座席番号
    let next_seat = sqlx::query!(
        r#"
        SELECT COALESCE(MAX(seat_number), 0) + 1 as "next_seat!"
        FROM reservations
        WHERE trip_id = $1
        "#,
        payload.trip_id
    )
    .fetch_one(&pool)
    .await
    .map_err(|e| {
        println!("DBエラー(座席計算): {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?
    .next_seat;

    // 定員チェック
    if next_seat > capacity {
        println!("❌ 満席です: 次の席 {}, 定員 {}", next_seat, capacity);
        return Err(StatusCode::UNPROCESSABLE_ENTITY);  // 422(Unprocessable Entity)
    }

    // 予約を保存
    let result = sqlx::query!(
        r#"
        INSERT INTO reservations (trip_id, user_id, seat_number)
        VALUES ($1, $2, $3)
        RETURNING reservation_id
        "#,
        payload.trip_id,
        payload.user_id,
        next_seat
    )
    .fetch_one(&pool)
    .await;

    match result {
        Ok(_rec) => {
            println!("✅ 予約完了! Seat: {} / Capacity: {}", next_seat, capacity);
            Ok(format!("予約が完了しました！ {}人目 (定員: {}名)", next_seat, capacity))
        }
        Err(e) => {
            println!("❌ 予約失敗: {:?}", e);
            // エラーの種類をチェックする
            // PostgresのUnique Violationエラーコードは "23505"
            if let Some(db_error) = e.as_database_error() {
                if db_error.code().as_deref() == Some("23505") {
                     return Err(StatusCode::CONFLICT); // 409: すでに予約済み
                }
            }
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
