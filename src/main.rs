use axum::{
    extract::State,
    http::StatusCode,
    routing::{delete, get, post},
    Json, Router,
};
use chrono::Local;
use clap::Parser;
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    path::Path,
    sync::Arc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tokio::{
    fs::{read_to_string, write},
    spawn,
    sync::Mutex,
};
use tower_http::trace::TraceLayer;
use tracing::{debug, error};
use tracing_subscriber::{
    fmt::{
        format::{Compact, DefaultFields},
        time::ChronoLocal,
    },
    layer::SubscriberExt,
    util::SubscriberInitExt as _,
    EnvFilter,
};

#[derive(Serialize)]
struct Claims {
    iss: String,
    scope: String,
    aud: String,
    exp: u64,
    iat: u64,
}

fn generate_jwt(private_key: &str, client_email: &str) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs();

    let claims = Claims {
        iss: client_email.to_string(),
        scope: "https://www.googleapis.com/auth/firebase.messaging".to_string(),
        aud: "https://oauth2.googleapis.com/token".to_string(),
        exp: now + 3600,
        iat: now,
    };

    let encoding_key =
        EncodingKey::from_rsa_pem(private_key.as_bytes()).expect("Failed to read private key");

    encode(&Header::new(Algorithm::RS256), &claims, &encoding_key).expect("Failed to encode JWT")
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
}

async fn get_oauth_token(jwt: &str) -> String {
    let client = Client::new();
    let params = [
        ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
        ("assertion", jwt),
    ];

    let response = client
        .post("https://oauth2.googleapis.com/token")
        .form(&params)
        .send()
        .await
        .expect("Failed to send request");

    let token_response: TokenResponse = response
        .json()
        .await
        .expect("Failed to parse token response");

    token_response.access_token
}

type AppStateType = Arc<Mutex<AppState>>;

struct AppState {
    recipient_token: String,
    oauth_token: String,
    notifications: Vec<Notification>,
    callsign: String,
    vpilot_connected: bool,
    alarm: Option<Instant>,
}

impl AppState {
    async fn send_fcm_message(&self, data: serde_json::Value) {
        let client = Client::new();
        let message = json!({
            "message": {
                "token": self.recipient_token,
                "data": data,
                "webpush": {
                    "headers": {
                        "Urgency": "high"
                    }
                },
                "android":{
                    "priority": "high"
                },
                "apns": {
                    "headers": {
                        "apns-priority": "10"
                    }
                },
            }
        });

        let response = client
            .post("https://fcm.googleapis.com/v1/projects/vpilot-alert/messages:send")
            .bearer_auth(self.oauth_token.clone())
            .json(&message)
            .send()
            .await
            .expect("Failed to send FCM message");

        if response.status().is_success() {
            debug!("FCM message sent successfully!");
        } else {
            debug!(
                "Failed to send FCM message: {}",
                response.text().await.expect("Failed to read response text")
            );
        }
    }
}

#[derive(Debug, Clone, Serialize)]
enum NotificationType {
    PrivateMessage,
    RadioMessage,
    SelcalAlert,
}

#[derive(Debug, Clone, Serialize)]
struct Notification {
    message: String,
    timestamp: String,
    _type: NotificationType,
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Your callsign
    #[arg(short, long)]
    callsign: String,

    /// Interface to run server on
    #[arg(short, long, default_value = "0.0.0.0:8080")]
    interface: String,
}

#[derive(Deserialize)]
struct GoogleServices {
    private_key: String,
    client_email: String,
}

fn get_layer<S>(
    layer: tracing_subscriber::fmt::Layer<S>,
) -> tracing_subscriber::fmt::Layer<
    S,
    DefaultFields,
    tracing_subscriber::fmt::format::Format<Compact, ChronoLocal>,
> {
    layer
        .with_timer(ChronoLocal::new("%v %k:%M:%S %z".to_owned()))
        .compact()
}

#[tokio::main]
async fn main() {
    let log_level = std::env::var("LOG").unwrap_or("warn".to_owned());
    tracing_subscriber::registry()
        .with(
            EnvFilter::new(format!("vpilot_alert={log_level}"))
                .add_directive(format!("tower_http::trace={log_level}").parse().unwrap()),
        )
        .with(get_layer(tracing_subscriber::fmt::layer()))
        .init();

    let args = Args::parse();

    let token_path = Path::new("token");
    let token = if token_path.exists() {
        read_to_string(token_path)
            .await
            .expect("Failed to read token file")
    } else {
        String::new()
    };

    let google_services: GoogleServices = serde_json::from_str(
        &read_to_string("google-services.json")
            .await
            .expect("Failed to read google-services.json"),
    )
    .expect("Failed to parse google-services.json");
    let jwt = generate_jwt(&google_services.private_key, &google_services.client_email);
    let oauth_token = get_oauth_token(&jwt).await;

    let app_state = Arc::new(Mutex::new(AppState {
        recipient_token: token,
        oauth_token,
        notifications: Vec::new(),
        callsign: args.callsign,
        vpilot_connected: true,
        alarm: None,
    }));
    let api_router = Router::new()
        .route("/fcm-token", post(save_token))
        .route("/private-message", post(private_message))
        .route("/radio-message", post(radio_message))
        .route("/selcal", post(selcal_alert))
        .route(
            "/connection-status",
            delete(set_disconnect_vpilot).get(get_connection_status),
        )
        .route("/notifications", get(get_notifications))
        .route("/alarm", delete(stop_alarm))
        .with_state(app_state.clone());

    let app = Router::new()
        .nest("/vpilot-alert/api/", api_router)
        .layer(TraceLayer::new_for_http())
        .fallback(handler_404);

    spawn(async move {
        loop {
            let mut state = app_state.lock().await;
            if let Some(alarm) = state.alarm {
                if alarm.elapsed() > Duration::from_secs(180) {
                    state.vpilot_connected = false;
                    state.alarm = None;
                    error!("Alarm time exceeded, disconnecting from vatsim");
                }
            }
            drop(state);
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    });

    debug!("Starting server on 8080");

    let listener = tokio::net::TcpListener::bind(args.interface).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

#[derive(Deserialize)]
struct TokenPayload {
    token: String,
}

async fn handler_404() -> impl axum::response::IntoResponse {
    (StatusCode::NOT_FOUND, "nothing to see here")
}

async fn save_token(state: State<AppStateType>, Json(payload): Json<TokenPayload>) -> StatusCode {
    let mut state = state.lock().await;
    state.recipient_token = payload.token;
    write("token", &*state.recipient_token)
        .await
        .expect("Failed to write token file");
    debug!("Token saved: {}", state.recipient_token);
    StatusCode::OK
}

#[derive(Deserialize)]
struct PrivateMessage {
    from: String,
    message: String,
}
async fn private_message(
    state: State<AppStateType>,
    Json(payload): Json<PrivateMessage>,
) -> StatusCode {
    let mut state = state.lock().await;
    state
        .send_fcm_message(json!({ "triggerAlarm": "true" }))
        .await;
    state.notifications.push(Notification {
        message: payload.message,
        timestamp: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        _type: NotificationType::PrivateMessage,
    });
    state.alarm = Some(Instant::now());
    StatusCode::OK
}

#[derive(Deserialize)]
struct RadioMessage {
    frequencies: Vec<i32>,
    from: String,
    message: String,
}

async fn radio_message(
    state: State<AppStateType>,
    Json(payload): Json<RadioMessage>,
) -> StatusCode {
    let mut state = state.lock().await;
    if payload
        .message
        .to_lowercase()
        .contains(state.callsign.to_lowercase().as_str())
    {
        state
            .send_fcm_message(json!({ "triggerAlarm": "true" }))
            .await;
        state.notifications.push(Notification {
            message: format!("Radio message received from {}", payload.from),
            timestamp: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            _type: NotificationType::RadioMessage,
        });
        state.alarm = Some(Instant::now());
    }
    StatusCode::OK
}

#[derive(Deserialize)]
struct SelcalAlert {
    frequencies: Vec<i32>,
    from: String,
}
async fn selcal_alert(state: State<AppStateType>, Json(payload): Json<SelcalAlert>) -> StatusCode {
    let mut state = state.lock().await;
    state
        .send_fcm_message(json!({ "triggerAlarm": "true" }))
        .await;
    state.notifications.push(Notification {
        message: format!("SELCAL received from {}", payload.from),
        timestamp: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        _type: NotificationType::SelcalAlert,
    });
    state.alarm = Some(Instant::now());
    StatusCode::OK
}

async fn set_disconnect_vpilot(state: State<AppStateType>) -> StatusCode {
    state.lock().await.vpilot_connected = false;
    StatusCode::OK
}

async fn get_connection_status(state: State<AppStateType>) -> Json<bool> {
    Json(state.lock().await.vpilot_connected)
}

async fn get_notifications(state: State<AppStateType>) -> Json<Vec<Notification>> {
    let state = state.lock().await;
    Json(state.notifications.clone())
}

async fn stop_alarm(state: State<AppStateType>) -> StatusCode {
    let mut state = state.lock().await;
    if state.alarm.is_some() {
        state.alarm = None;
    }

    StatusCode::OK
}
