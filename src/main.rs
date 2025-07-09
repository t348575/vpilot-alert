use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get, post},
    Json, Router,
};
use clap::Parser;
use serde::Deserialize;
use std::sync::Arc;
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

use crate::{
    fcm::GoogleServices,
    route::{Route, RouteStatistics},
    state::{AppState, AppStateType, Notification, NotificationType},
};

mod fcm;
mod route;
mod state;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Your callsign
    #[arg(short, long)]
    callsign: String,

    /// Navigation database path
    #[arg(short, long)]
    nav_db_path: String,

    /// Interface to run server on
    #[arg(short, long, default_value = "0.0.0.0:8080")]
    interface: String,
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

    let token_path = std::path::Path::new("token");
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
    google_services
        .login()
        .await
        .expect("Failed to login to google services");

    let mut route = Route::new(&args.nav_db_path, &args.callsign).expect("Failed to create route");
    let stats = route
        .route_statistics()
        .await
        .expect("Failed to get route statistics");

    let app_state = Arc::new(Mutex::new(AppState {
        recipient_token: token,
        google_services,
        notifications: Vec::new(),
        callsign: args.callsign,
        vpilot_connected: true,
        alarm: None,
        stats,
        route,
        alert_crashes: false,
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
        .route(
            "/notifications",
            get(get_notifications).delete(clear_notifications),
        )
        .route("/alert_crashes/{alert_crashes}", post(set_alert_crashes))
        .route("/alert_crashes", get(get_alert_crashes))
        .route("/stats", get(get_stats))
        .route("/alarm", delete(stop_alarm).post(received_alarm))
        .route("/notify", post(send_notification))
        .with_state(app_state.clone());

    let app = Router::new()
        .nest("/vpilot-alert/api/", api_router)
        .layer(TraceLayer::new_for_http())
        .fallback(handler_404);

    spawn(AppState::state_loop(app_state));

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
    if let Err(err) = state
        .send_notification(
            format!("{}: {}", payload.from, payload.message),
            NotificationType::PrivateMessage,
        )
        .await
    {
        error!("Failed to send notification: {}", err);
    }
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
        if let Err(err) = state
            .send_notification(
                format!(
                    "{} @ {:?}: {}",
                    payload.from, payload.frequencies, payload.message
                ),
                NotificationType::RadioMessage,
            )
            .await
        {
            error!("Failed to send notification: {}", err);
        }
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
    if let Err(err) = state
        .send_notification(
            format!("SELCAL {} @ {:?}", payload.from, payload.frequencies),
            NotificationType::SelcalAlert,
        )
        .await
    {
        error!("Failed to send notification: {}", err);
    }
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
async fn clear_notifications(state: State<AppStateType>) -> StatusCode {
    let mut state = state.lock().await;
    state.notifications.clear();
    StatusCode::OK
}

async fn stop_alarm(state: State<AppStateType>) -> StatusCode {
    let mut state = state.lock().await;
    if state.alarm.is_some() {
        state.alarm = None;
    }

    StatusCode::OK
}

async fn received_alarm(state: State<AppStateType>) -> StatusCode {
    let mut state = state.lock().await;
    if let Some(alarm) = &mut state.alarm {
        alarm.alarm_played = true;
    };
    StatusCode::OK
}

#[derive(Deserialize)]
struct SetAlertCrashes {
    alert_crashes: bool,
}

async fn set_alert_crashes(
    Path(SetAlertCrashes { alert_crashes }): Path<SetAlertCrashes>,
    state: State<AppStateType>,
) -> StatusCode {
    let mut state = state.lock().await;
    state.alert_crashes = alert_crashes;
    StatusCode::OK
}

async fn get_alert_crashes(state: State<AppStateType>) -> Json<bool> {
    let state = state.lock().await;
    Json(state.alert_crashes)
}

async fn get_stats(state: State<AppStateType>) -> Json<RouteStatistics> {
    let state = state.lock().await;
    Json(state.stats.clone())
}

async fn send_notification(state: State<AppStateType>) -> StatusCode {
    let mut state = state.lock().await;
    if let Err(err) = state
        .send_notification(
            "Test notification".to_string(),
            NotificationType::PrivateMessage,
        )
        .await
    {
        error!("Failed to send notification: {}", err);
    }
    StatusCode::OK
}
