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
        .route("/my-reservations", post(get_my_reservations))
        .route("/reservations/cancel", post(cancel_reservation))
        .route("/admin/status", post(insert_status))
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
    role: String,
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

#[derive(Serialize)]
struct MyReservationResponse {
    reservation_id: uuid::Uuid,
    trip_id: uuid::Uuid,
    seat_number: i32,
    departure_time: NaiveDateTime,
    source: String,
    destination: String,
    vehicle_name: String,
}

#[derive(Deserialize)]
struct CancelReservationRequest {
    reservation_id: uuid::Uuid,
    user_id: uuid::Uuid,
}

#[derive(Deserialize)]
struct InsertStatusRequest {
    user_id: uuid::Uuid,     // 権限チェック
    trip_id: uuid::Uuid,
    status: String, // "delayed", "cancelled"
    description: Option<String>,
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
        r#"
        SELECT user_id, name, password, role as "role!: String"
        FROM users
        WHERE email = $1
        "#,
        payload.email
    )
    .fetch_optional(&pool)
    .await
    .map_err(|e| {
        println!("DBエラー: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // ユーザーが存在するかチェック
    let user = match user {
        Some(u) => u,
        None => {
            println!("ユーザーが見つかりません: {}", payload.email);
            return Err(StatusCode::UNAUTHORIZED); // 401 Unauthorized
        }
    };

    // パスワードが合っているかチェック (verify)
    // payload.password (入力された平文) と user.password (DBのハッシュ) を比較
    let is_valid = verify(payload.password, &user.password)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if is_valid {
        println!("ログイン成功: {}", user.name);

        let response = LoginResponse {
            user_id: user.user_id,
            name: user.name,
            role: user.role,
        };
        Ok(Json(response))
    } else {
        println!("パスワード不一致: {}", payload.email);
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
        println!("DBエラー: {:?}", e);
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
        println!("満席です: 次の席 {}, 定員 {}", next_seat, capacity);
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
            println!("予約完了! Seat: {} / Capacity: {}", next_seat, capacity);
            Ok(format!("予約が完了しました！ {}人目 (定員: {}名)", next_seat, capacity))
        }
        Err(e) => {
            println!("予約失敗: {:?}", e);
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

// 自分の予約一覧取得 (POST /my-reservations)
#[derive(Deserialize)]
struct GetMyReservationsRequest {
    user_id: uuid::Uuid,
}

async fn get_my_reservations(
    State(pool): State<PgPool>,
    Json(payload): Json<GetMyReservationsRequest>,
) -> Result<Json<Vec<MyReservationResponse>>, StatusCode> {

    let rows = sqlx::query!(
        r#"
        SELECT
            r.reservation_id,
            r.seat_number,
            t.trip_id,
            t.departure_datetime,
            s_stop.name as "source_name!",
            d_stop.name as "dest_name!",
            v.vehicle_name as "vehicle_name!"
        FROM reservations r
        JOIN trips t ON r.trip_id = t.trip_id
        JOIN routes rt ON t.route_id = rt.route_id
        JOIN bus_stops s_stop ON rt.source_bus_stop_id = s_stop.bus_stop_id
        JOIN bus_stops d_stop ON rt.destination_bus_stop_id = d_stop.bus_stop_id
        JOIN vehicles v ON t.vehicle_id = v.vehicle_id
        WHERE r.user_id = $1
        ORDER BY t.departure_datetime DESC
        "#,
        payload.user_id
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| {
        println!("DBエラー: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let reservations = rows.into_iter().map(|row| MyReservationResponse {
        reservation_id: row.reservation_id,
        trip_id: row.trip_id,
        seat_number: row.seat_number,
        departure_time: row.departure_datetime,
        source: row.source_name,
        destination: row.dest_name,
        vehicle_name: row.vehicle_name,
    }).collect();

    Ok(Json(reservations))
}

// 予約キャンセル (POST /reservations/cancel)
async fn cancel_reservation(
    State(pool): State<PgPool>,
    Json(payload): Json<CancelReservationRequest>,
) -> Result<String, StatusCode> {
    println!("【キャンセル】Reservation: {}, User: {}", payload.reservation_id, payload.user_id);

    // WHERE user_id = $2 をつけることで、「他人の予約」を勝手に消せない
    let result = sqlx::query!(
        "DELETE FROM reservations WHERE reservation_id = $1 AND user_id = $2",
        payload.reservation_id,
        payload.user_id
    )
    .execute(&pool)
    .await
    .map_err(|e| {
        println!("DBエラー: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // 削除された行があるかチェック
    if result.rows_affected() == 0 {
        // 0行だった場合＝「予約IDが存在しない」か「ユーザーIDが一致しない（他人の予約）」
        println!("キャンセル失敗（対象なし）");
        return Err(StatusCode::NOT_FOUND); // 404 Not Found
    }

    println!("キャンセル成功");
    Ok("予約をキャンセルしました".to_string())
}



// 運行状況の登録・更新 (POST /admin/status)
async fn insert_status(
    State(pool): State<PgPool>,
    Json(payload): Json<InsertStatusRequest>,
) -> Result<String, StatusCode> {
    println!("【管理者】運行状況変更: User={}, Trip={}, Status={}", payload.user_id, payload.trip_id, payload.status);

    // 1. 権限チェック (Adminかどうか)
    let user = sqlx::query!(
        "SELECT role as \"role!: String\" FROM users WHERE user_id = $1",
        payload.user_id
    )
    .fetch_optional(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    match user {
        Some(u) if u.role == "admin" => {}, // OK
        _ => return Err(StatusCode::FORBIDDEN),
    }

    // 2. ステータスによって処理を分岐！
    match payload.status.as_str() {
        // ★平常 (scheduled) の場合 -> レコードを削除する（＝平常に戻す）
        "scheduled" => {
            let result = sqlx::query!(
                "DELETE FROM operational_statuses WHERE trip_id = $1",
                payload.trip_id
            )
            .execute(&pool)
            .await;

            match result {
                Ok(_) => {
                    println!("✅ 平常運転に戻しました（レコード削除）");
                    return Ok("運行状況を '通常' に戻しました".to_string());
                }
                Err(e) => {
                    println!("❌ DBエラー: {:?}", e);
                    return Err(StatusCode::INTERNAL_SERVER_ERROR);
                }
            }
        },

        // ★遅延 (delayed) または 運休 (cancelled) の場合 -> レコードを保存・更新する
        "delayed" | "cancelled" => {
            let result = sqlx::query!(
                r#"
                INSERT INTO operational_statuses (trip_id, status, description)
                VALUES ($1, $2::text::trip_status, $3)
                ON CONFLICT (trip_id)
                DO UPDATE SET
                    status = EXCLUDED.status,
                    description = EXCLUDED.description,
                    updated_at = NOW()
                "#,
                payload.trip_id,
                payload.status,
                payload.description
            )
            .execute(&pool)
            .await;

            match result {
                Ok(_) => {
                    println!("✅ 状況更新成功: {}", payload.status);
                    send_teams_notification(&pool, payload.trip_id, &payload.status, &payload.description).await;
                    Ok(format!("運行状況を '{}' に変更しました", payload.status))
                }
                Err(e) => {
                    println!("❌ DBエラー: {:?}", e);
                    Err(StatusCode::INTERNAL_SERVER_ERROR)
                }
            }
        },

        // それ以外（変な文字）
        _ => return Err(StatusCode::BAD_REQUEST),
    }
}



// Teams通知機能
async fn send_teams_notification(
    pool: &PgPool,
    trip_id: uuid::Uuid,
    status: &str,
    description: &Option<String>,
) {
    // 1. 環境変数からURLを取得
    let webhook_url = match std::env::var("TEAMS_WEBHOOK_URL") {
        Ok(url) => url,
        Err(_) => {
            println!("⚠️ TEAMS_WEBHOOK_URLが設定されていないため通知をスキップします");
            return;
        }
    };

    // 2. その便を予約しているユーザー(メールと名前)を取得
    struct UserInfo { name: String, email: String }
    let users = sqlx::query_as!(
        UserInfo,
        r#"
        SELECT u.name, u.email
        FROM reservations r
        JOIN users u ON r.user_id = u.user_id
        WHERE r.trip_id = $1
        "#,
        trip_id
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default(); // エラーなら空リスト

    if users.is_empty() {
        println!("ℹ️ 予約者がいないため通知しません");
        return;
    }

    // 3. メンション用のデータを作る
    // Teamsのメンションには "<at>名前</at>" というテキストと、
    // それに対応する "mentioned": { "id": "email", ... } というデータが必要です。

    let mut mention_text_parts = Vec::new();
    let mut mention_entities = Vec::new();

    for user in users {
        // テキスト部分: <at>高専太郎</at>
        let text_tag = format!("<at>{}</at>", user.name);
        mention_text_parts.push(text_tag.clone());

        // データ部分
        mention_entities.push(serde_json::json!({
            "type": "mention",
            "text": text_tag,
            "mentioned": {
                "id": user.email, // ここがTeamsの登録メアドと一致していれば通知が飛ぶ
                "name": user.name
            }
        }));
    }

    let all_mentions_str = mention_text_parts.join(" ");
    let status_msg = match status {
        "delayed" => "⚠️ 【遅延情報】",
        "cancelled" => "kB 【運休情報】", // kBは赤いアイコンっぽいやつ
        _ => "【運行情報】"
    };
    let desc_str = description.clone().unwrap_or("詳細は管理画面を確認してください".to_string());

    // 4. Adaptive Card の JSON を組み立てる
    let payload = serde_json::json!({
        "type": "message",
        "attachments": [
            {
                "contentType": "application/vnd.microsoft.card.adaptive",
                "content": {
                    "type": "AdaptiveCard",
                    "body": [
                        {
                            "type": "TextBlock",
                            "size": "Medium",
                            "weight": "Bolder",
                            "text": format!("{} 産技往復便のお知らせ", status_msg),
                            "color": if status == "cancelled" { "Attention" } else { "Warning" }
                        },
                        {
                            "type": "TextBlock",
                            "text": format!("以下の便の運行状況が **{}** に変更されました。", status.to_uppercase()),
                            "wrap": true
                        },
                        {
                            "type": "FactSet",
                            "facts": [
                                { "title": "詳細:", "value": desc_str }
                            ]
                        },
                        {
                            "type": "TextBlock",
                            "text": "対象者への通知:",
                            "weight": "Bolder",
                            "spacing": "Medium"
                        },
                        {
                            "type": "TextBlock",
                            "text": all_mentions_str, // ここに <at>...が入る
                            "wrap": true
                        }
                    ],
                    "$schema": "http://adaptivecards.io/schemas/adaptive-card.json",
                    "version": "1.2",
                    "msteams": {
                        "entities": mention_entities // ここに実データが入る
                    }
                }
            }
        ]
    });

    // 5. 送信
    let client = reqwest::Client::new();
    match client.post(&webhook_url).json(&payload).send().await {
        Ok(_) => println!("✅ Teams通知送信成功"),
        Err(e) => println!("❌ Teams通知送信失敗: {:?}", e),
    }
}
