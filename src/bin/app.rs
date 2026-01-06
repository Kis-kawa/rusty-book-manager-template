use axum::{
    Json, Router, extract::{Path, State}, http::{Method, StatusCode}, routing::{delete, get, post}
};
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tokio::time::{self, Duration};
use tower_http::cors::{CorsLayer, Any};
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use bcrypt::{hash, verify, DEFAULT_COST};
use chrono::{Local, NaiveDateTime};

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
        .route("/admin/options", post(get_admin_options)) // 権限チェックのためPOSTにします
        .route("/admin/trips", post(create_trip))
        .route("/admin/reservations/:reservation_id", delete(admin_delete_reservation))
        .route("/admin/maintenance", get(get_maintenance_status).post(set_maintenance_status))
        .layer(cors)
        .with_state(pool.clone());

    let cron_pool = pool.clone();
    tokio::spawn(async move {
        run_cron_job(cron_pool).await;
    });

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

// 管理者用：マスターデータ取得 (GET /admin/options) 用
#[derive(Serialize)]
struct RouteOption {
    route_id: uuid::Uuid,
    name: String, // "品川 -> 荒川"
}

#[derive(Serialize)]
struct SimpleOption {
    id: uuid::Uuid,
    name: String,
}

#[derive(Serialize)]
struct AdminOptionsResponse {
    routes: Vec<RouteOption>,
    vehicles: Vec<SimpleOption>,
    drivers: Vec<SimpleOption>,
}

// 管理者用：便作成 (POST /admin/trips) 用
#[derive(Deserialize)]
struct CreateTripRequest {
    user_id: uuid::Uuid, // 権限チェック用
    route_id: uuid::Uuid,
    vehicle_id: uuid::Uuid,
    driver_id: uuid::Uuid,
    departure_datetime: NaiveDateTime,
    arrival_datetime: NaiveDateTime,
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
) -> Result<(StatusCode, String), StatusCode> {
    println!("【予約】Trip: {}, User: {}", payload.trip_id, payload.user_id);

    if is_maintenance_mode(&pool).await {
        println!("⛔️ メンテナンス中のため予約を拒否しました");
        // 503 Service Unavailable を返す
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    }

    // status が 'cancelled' なら予約させない
    let trip = sqlx::query!(
        r#"
        SELECT
            t.departure_datetime,
            os.status as "status?: String" -- LEFT JOINなのでNULLの可能性あり
        FROM trips t
        LEFT JOIN operational_statuses os ON t.trip_id = os.trip_id
        WHERE t.trip_id = $1
        "#,
        payload.trip_id
    )
    .fetch_optional(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let trip = match trip {
        Some(t) => {
            // ★追加: 運休チェック
            if let Some(ref status) = t.status {
                if status == "cancelled" {
                    return Err(StatusCode::SERVICE_UNAVAILABLE); // 503エラーを返す
                }
            }
            t
        },
        None => return Err(StatusCode::NOT_FOUND),
    };

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
        Ok(_) => {
            println!("✅ 予約作成成功");

            // 駆け込み予約チェック
            // 出発まで2時間を切っているかチェックする
            let now = Local::now().naive_local();
            // trip.departure_datetime と現在の差分を計算
            let duration_until_departure = trip.departure_datetime - now;

            // 「未来の出発」かつ「2時間(120分)以内」なら即時通知
            if duration_until_departure.num_seconds() > 0 && duration_until_departure.num_minutes() <= 120 {
                println!("🏃💨 出発2時間以内の駆け込み予約を検知！リマインドを送ります。");

                let pool_clone = pool.clone();
                let trip_id = payload.trip_id;
                let user_id = payload.user_id;

                // 別スレッドで通知を送る
                tokio::spawn(async move {
                    send_personal_reminder(&pool_clone, trip_id, user_id).await;
                });
            }

            Ok((StatusCode::CREATED, "予約しました".to_string()))
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

                    // 非同期で通知 ＆ キャンセル処理
                    let pool_clone = pool.clone();
                    let trip_id = payload.trip_id;
                    let status = payload.status.clone(); // "cancelled" かどうか判定に使う
                    let description = payload.description.clone();

                    tokio::spawn(async move {
                        // 1. まず通知を送る（この時点ではまだ予約データが必要！）
                        send_teams_notification(&pool_clone, trip_id, &status, &description).await;

                        // 2. 「運休」の場合のみ、通知後に予約を全削除する
                        if status == "cancelled" {
                            println!("🗑️ 運休のため予約データを削除します: {}", trip_id);

                            let delete_result = sqlx::query!(
                                "DELETE FROM reservations WHERE trip_id = $1",
                                trip_id
                            )
                            .execute(&pool_clone)
                            .await;

                            match delete_result {
                                Ok(res) => println!("✅ 予約削除完了: {}件", res.rows_affected()),
                                Err(e) => println!("❌ 予約削除失敗: {:?}", e),
                            }
                        }
                    });

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


// マスタデータ一括取得 (POST /admin/options)
#[derive(Deserialize)]
struct AdminAuthRequest {
    user_id: uuid::Uuid,
}

async fn get_admin_options(
    State(pool): State<PgPool>,
    Json(payload): Json<AdminAuthRequest>,
) -> Result<Json<AdminOptionsResponse>, StatusCode> {
    // 権限チェック
    let user = sqlx::query!("SELECT role as \"role!: String\" FROM users WHERE user_id = $1", payload.user_id)
        .fetch_optional(&pool).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // roleが取れない、またはadminでない場合はエラー
    match user {
        Some(u) if u.role == "admin" => {},
        _ => return Err(StatusCode::FORBIDDEN),
    }

    // ルート一覧取得 (品川->荒川 のように名前を結合)
    let routes = sqlx::query!(
        r#"
        SELECT
            r.route_id,
            s.name as "source!",
            d.name as "dest!"
        FROM routes r
        JOIN bus_stops s ON r.source_bus_stop_id = s.bus_stop_id
        JOIN bus_stops d ON r.destination_bus_stop_id = d.bus_stop_id
        "#
    )
    .fetch_all(&pool).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // 車両一覧取得
    let vehicles = sqlx::query!("SELECT vehicle_id, vehicle_name FROM vehicles")
        .fetch_all(&pool).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // 運転手一覧取得
    let drivers = sqlx::query!("SELECT driver_id, name FROM drivers")
        .fetch_all(&pool).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // レスポンス作成
    Ok(Json(AdminOptionsResponse {
        routes: routes.into_iter().map(|r| RouteOption {
            route_id: r.route_id,
            name: format!("{} → {}", r.source, r.dest),
        }).collect(),
        vehicles: vehicles.into_iter().map(|v| SimpleOption {
            id: v.vehicle_id,
            name: v.vehicle_name,
        }).collect(),
        drivers: drivers.into_iter().map(|d| SimpleOption {
            id: d.driver_id,
            name: d.name,
        }).collect(),
    }))
}


// 便の新規作成 (POST /admin/trips)
async fn create_trip(
    State(pool): State<PgPool>,
    Json(payload): Json<CreateTripRequest>,
) -> Result<String, StatusCode> {
    println!("【管理者】新規便作成リクエスト");

    // 権限チェック
    let user = sqlx::query!("SELECT role as \"role!: String\" FROM users WHERE user_id = $1", payload.user_id)
        .fetch_optional(&pool).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    match user {
        Some(u) if u.role == "admin" => {},
        _ => return Err(StatusCode::FORBIDDEN),
    }

    // tripsテーブルにINSERT
    // trip_date は departure_datetime の日付部分を自動で採用します
    let result = sqlx::query!(
        r#"
        INSERT INTO trips (route_id, vehicle_id, driver_id, trip_date, departure_datetime, arrival_datetime)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
        payload.route_id,
        payload.vehicle_id,
        payload.driver_id,
        payload.departure_datetime.date(), // $4: 日付だけを取り出して渡す (NaiveDate)
        payload.departure_datetime,        // $5: 日時そのまま (NaiveDateTime)
        payload.arrival_datetime           // $6: 日時そのまま
    )
    .execute(&pool)
    .await;

    match result {
        Ok(_) => {
            println!("便作成成功");
            Ok("新しい便を作成しました".to_string())
        }
        Err(e) => {
            println!("DBエラー: {:?}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}


// ----------------------------------------------------------------
// 通知タスク
// ----------------------------------------------------------------


// Teams通知機能
async fn send_teams_notification(
    pool: &PgPool,
    trip_id: uuid::Uuid,
    status: &str,
    description: &Option<String>,
) {
    let webhook_url = match std::env::var("TEAMS_WEBHOOK_URL") {
        Ok(url) => url,
        Err(_) => {
            println!("TEAMS_WEBHOOK_URLが設定されていないため通知をスキップします");
            return;
        }
    };

    struct TripInfo {
        source: String,
        destination: String,
        departure_time: NaiveDateTime,
        vehicle_name: String,
    }

    // 便の詳細情報を取得
    let trip_info = sqlx::query_as!(
        TripInfo,
        r#"
        SELECT
            s.name as "source!",
            d.name as "destination!",
            t.departure_datetime as departure_time,
            v.vehicle_name as "vehicle_name!"
        FROM trips t
        JOIN routes r ON t.route_id = r.route_id
        JOIN bus_stops s ON r.source_bus_stop_id = s.bus_stop_id
        JOIN bus_stops d ON r.destination_bus_stop_id = d.bus_stop_id
        JOIN vehicles v ON t.vehicle_id = v.vehicle_id
        WHERE t.trip_id = $1
        "#,
        trip_id
    )
    .fetch_optional(pool)
    .await
    .unwrap_or(None);

    let trip_details_text = match trip_info {
        Some(info) => format!(
            "{} {}発\n{} → {}",
            info.departure_time.format("%m/%d %H:%M"),
            info.vehicle_name,
            info.source,
            info.destination
        ),
        None => "便情報の取得に失敗しました".to_string(),
    };

    // 予約者の取得
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
    .unwrap_or_default();

    if users.is_empty() {
        println!("予約者がいないため通知しません");
        return;
    }

    // メンションデータの作成
    let mut mention_text_parts = Vec::new();
    let mut mention_entities = Vec::new();

    for user in users {
        let text_tag = format!("<at>{}</at>", user.name);
        let display_text = format!("{} 様", text_tag);

        mention_text_parts.push(display_text);

        mention_entities.push(serde_json::json!({
            "type": "mention",
            "text": text_tag,
            "mentioned": {
                "id": user.email,
                "name": user.name
            }
        }));
    }

    let all_mentions_str = mention_text_parts.join("　");

    // 表示テキストの整備
    let (status_title, status_color, status_text_jp) = match status {
        "delayed" => ("⚠️ 【遅延情報】", "Warning", "遅延"),
        "cancelled" => ("🚫 【運休情報】", "Attention", "運休"),
        _ => ("【運行情報】", "Accent", "変更"),
    };

    let desc_str = description.clone().unwrap_or("詳細は管理画面を確認してください".to_string());

    // Adaptive Card JSON
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
                            "text": format!("{} 産技往復便のお知らせ", status_title),
                            "color": status_color
                        },
                        {
                            "type": "TextBlock",
                            "text": format!("以下の便の運行状況が **{}** に変更されました。", status_text_jp),
                            "wrap": true
                        },
                        {
                            "type": "FactSet",
                            "facts": [
                                { "title": "対象便:", "value": trip_details_text },
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
                            "text": all_mentions_str,
                            "wrap": true
                        }
                    ],
                    "$schema": "http://adaptivecards.io/schemas/adaptive-card.json",
                    "version": "1.2",
                    "msteams": {
                        "entities": mention_entities
                    }
                }
            }
        ]
    });

    // 送信
    let client = reqwest::Client::new();
    match client.post(&webhook_url).json(&payload).send().await {
        Ok(_) => println!("Teams通知送信成功"),
        Err(e) => println!("Teams通知送信失敗: {:?}", e),
    }
}


// リマインド通知送信関数（自動実行用）
async fn send_reminder_notification(pool: &PgPool, trip_id: uuid::Uuid) -> bool {
    // 便情報の取得
    struct TripData {
        source: String,
        destination: String,
        departure_time: NaiveDateTime,
        vehicle_name: String,
    }
    let trip = match sqlx::query_as!(
        TripData,
        r#"
        SELECT
            s.name as "source!", d.name as "destination!",
            t.departure_datetime as departure_time, v.vehicle_name as "vehicle_name!"
        FROM trips t
        JOIN routes r ON t.route_id = r.route_id
        JOIN bus_stops s ON r.source_bus_stop_id = s.bus_stop_id
        JOIN bus_stops d ON r.destination_bus_stop_id = d.bus_stop_id
        JOIN vehicles v ON t.vehicle_id = v.vehicle_id
        WHERE t.trip_id = $1
        "#,
        trip_id
    )
    .fetch_optional(pool).await.unwrap_or(None) {
        Some(t) => t,
        None => return false, // 便がない
    };

    // 予約者の取得（重複除外）
    struct UserData { name: String, email: String }
    let users = sqlx::query_as!(
        UserData,
        r#"
        SELECT DISTINCT u.name, u.email
        FROM reservations r
        JOIN users u ON r.user_id = u.user_id
        WHERE r.trip_id = $1
        "#,
        trip_id
    )
    .fetch_all(pool).await.unwrap_or_default();

    // 予約者がいない場合は false を返す
    if users.is_empty() {
        println!("まだ予約者がいないため、リマインド通知を保留します: {}", trip.departure_time);
        return false;
    }

    // メンション作成
    let mut mention_text_parts = Vec::new();
    let mut mention_entities = Vec::new();
    for user in users {
        let text_tag = format!("<at>{}</at>", user.name);
        mention_text_parts.push(format!("{} 様", text_tag));
        mention_entities.push(serde_json::json!({
            "type": "mention", "text": text_tag,
            "mentioned": { "id": user.email, "name": user.name }
        }));
    }

    //  通知JSON作成
    let webhook_url = std::env::var("TEAMS_WEBHOOK_URL").unwrap_or_default();
    if webhook_url.is_empty() { return false; }

    let payload = serde_json::json!({
        "type": "message",
        "attachments": [{
            "contentType": "application/vnd.microsoft.card.adaptive",
            "content": {
                "type": "AdaptiveCard",
                "$schema": "http://adaptivecards.io/schemas/adaptive-card.json",
                "version": "1.2",
                "body": [
                    {
                        "type": "TextBlock",
                        "size": "Medium",
                        "weight": "Bolder",
                        "text": "⏰ まもなく出発時刻です",
                        "color": "Accent"
                    },
                    {
                        "type": "TextBlock",
                        "text": "ご予約のバスが**2時間以内**に出発します。乗り遅れのないようご注意ください。",
                        "wrap": true
                    },
                    {
                        "type": "FactSet",
                        "facts": [
                            { "title": "出発時刻:", "value": trip.departure_time.format("%H:%M").to_string() },
                            { "title": "区間:", "value": format!("{} → {}", trip.source, trip.destination) },
                            { "title": "車両:", "value": trip.vehicle_name }
                        ]
                    },
                    { "type": "TextBlock", "text": mention_text_parts.join("　"), "wrap": true }
                ],
                "msteams": { "entities": mention_entities }
            }
        }]
    });

    // 送信
    let _ = reqwest::Client::new().post(&webhook_url).json(&payload).send().await;
    println!("✅ リマインド通知送信完了: {}", trip.departure_time);

    true // 送信したので true
}

// ----------------------------------------------------------------
// 定期実行タスク (Cron Job)
// ----------------------------------------------------------------
async fn run_cron_job(pool: PgPool) {
    let mut interval = time::interval(Duration::from_secs(60));

    loop {
        interval.tick().await;

        let now = Local::now().naive_local();
        println!("🔍 [TimeCheck] アプリ現在時刻(JST): {}", now);

        let trips = sqlx::query!(
            r#"
            SELECT trip_id
            FROM trips
            WHERE departure_datetime > $1
              AND departure_datetime <= $1 + INTERVAL '2 hours'
              AND notification_sent = FALSE
            "#,
            now
        )
        .fetch_all(&pool)
        .await;

        if let Ok(trip_rows) = trips {
            for row in trip_rows {
                println!("🚀 リマインド対象発見: {}", row.trip_id);

                // A. 通知を送ってみる
                // ★修正: 戻り値(sent)を受け取る
                let sent = send_reminder_notification(&pool, row.trip_id).await;

                // B. 送信できた場合のみ「通知済み」マークをつける
                if sent {
                    let _ = sqlx::query!(
                        "UPDATE trips SET notification_sent = TRUE WHERE trip_id = $1",
                        row.trip_id
                    )
                    .execute(&pool)
                    .await;
                }
            }
        }
    }
}

// 管理者用：予約強制削除 (DELETE /admin/reservations/:id)
async fn admin_delete_reservation(
    State(pool): State<PgPool>,
    Path(reservation_id): Path<uuid::Uuid>,
    // ヘッダーなどで管理者権限チェックをするのが理想ですが、今回は簡易的に
) -> Result<String, StatusCode> {

    let result = sqlx::query!(
        "DELETE FROM reservations WHERE reservation_id = $1",
        reservation_id
    )
    .execute(&pool)
    .await;

    match result {
        Ok(res) => {
            if res.rows_affected() > 0 {
                Ok("予約を強制キャンセルしました".to_string())
            } else {
                Err(StatusCode::NOT_FOUND)
            }
        }
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}


// 個人宛リマインド通知（駆け込み予約用）
async fn send_personal_reminder(pool: &PgPool, trip_id: uuid::Uuid, user_id: uuid::Uuid) {
    // 1. 便情報の取得
    struct TripData {
        source: String, destination: String,
        departure_time: NaiveDateTime, vehicle_name: String,
    }
    let trip = match sqlx::query_as!(
        TripData,
        r#"
        SELECT s.name as "source!", d.name as "destination!",
               t.departure_datetime as departure_time, v.vehicle_name as "vehicle_name!"
        FROM trips t
        JOIN routes r ON t.route_id = r.route_id
        JOIN bus_stops s ON r.source_bus_stop_id = s.bus_stop_id
        JOIN bus_stops d ON r.destination_bus_stop_id = d.bus_stop_id
        JOIN vehicles v ON t.vehicle_id = v.vehicle_id
        WHERE t.trip_id = $1
        "#,
        trip_id
    ).fetch_optional(pool).await.unwrap_or(None) {
        Some(t) => t, None => return,
    };

    // 2. ユーザー情報の取得（対象の1名だけ）
    let user = match sqlx::query!(
        "SELECT name, email FROM users WHERE user_id = $1",
        user_id
    ).fetch_optional(pool).await.unwrap_or(None) {
        Some(u) => u, None => return,
    };

    // 3. Teams通知の作成 (メンション付き)
    let webhook_url = std::env::var("TEAMS_WEBHOOK_URL").unwrap_or_default();
    if webhook_url.is_empty() { return; }

    let text_tag = format!("<at>{}</at>", user.name);

    let payload = serde_json::json!({
        "type": "message",
        "attachments": [{
            "contentType": "application/vnd.microsoft.card.adaptive",
            "content": {
                "type": "AdaptiveCard", "$schema": "http://adaptivecards.io/schemas/adaptive-card.json", "version": "1.2",
                "body": [
                    { "type": "TextBlock", "size": "Medium", "weight": "Bolder", "text": "⏰ 出発直前のご予約です", "color": "Attention" },
                    { "type": "TextBlock", "text": "ご予約ありがとうございます。バスは**まもなく出発**します。", "wrap": true },
                    { "type": "FactSet", "facts": [
                        { "title": "出発時刻:", "value": trip.departure_time.format("%H:%M").to_string() },
                        { "title": "区間:", "value": format!("{} → {}", trip.source, trip.destination) },
                        { "title": "車両:", "value": trip.vehicle_name }
                    ]},
                    { "type": "TextBlock", "text": format!("{} 様", text_tag), "wrap": true }
                ],
                "msteams": { "entities": [{
                    "type": "mention", "text": text_tag,
                    "mentioned": { "id": user.email, "name": user.name }
                }]}
            }
        }]
    });

    // 4. 送信 (エラーハンドリングはログ出力のみ)
    let _ = reqwest::Client::new().post(&webhook_url).json(&payload).send().await;
    println!("⚡️ 駆け込み予約リマインド送信: {}", user.name);
}



// ----------------------------------------------------------------
// メンテナンスモード関連
// ----------------------------------------------------------------

// ヘルパー: 現在メンテナンス中かどうかDBを見る
async fn is_maintenance_mode(pool: &PgPool) -> bool {
    let row = sqlx::query!("SELECT value FROM app_settings WHERE key = 'maintenance_mode'")
        .fetch_optional(pool)
        .await
        .unwrap_or(None);

    if let Some(r) = row {
        r.value == "true"
    } else {
        false
    }
}

// API: メンテナンスモードの状態を取得 (GET /admin/maintenance)
async fn get_maintenance_status(State(pool): State<PgPool>) -> Result<Json<bool>, StatusCode> {
    let mode = is_maintenance_mode(&pool).await;
    Ok(Json(mode))
}

// API: メンテナンスモードの切り替え (POST /admin/maintenance)
#[derive(Deserialize)]
struct MaintenanceRequest {
    enabled: bool,
    user_id: uuid::Uuid, // 管理者チェック用
}

async fn set_maintenance_status(
    State(pool): State<PgPool>,
    Json(payload): Json<MaintenanceRequest>,
) -> Result<String, StatusCode> {
    // 1. 管理者権限チェック
    let user = sqlx::query!("SELECT role as \"role!: String\" FROM users WHERE user_id = $1", payload.user_id)
        .fetch_optional(&pool).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    match user {
        Some(u) if u.role == "admin" => {},
        _ => return Err(StatusCode::FORBIDDEN),
    }

    // 2. 設定更新
    let val_str = if payload.enabled { "true" } else { "false" };
    sqlx::query!(
        "UPDATE app_settings SET value = $1 WHERE key = 'maintenance_mode'",
        val_str
    )
    .execute(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    println!("🔧 メンテナンスモードを {} に変更しました", val_str);
    Ok("設定を変更しました".to_string())
}
