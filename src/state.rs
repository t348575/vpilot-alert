use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use chrono::Local;
use eyre::{Context, Result};
use serde::Serialize;
use serde_json::json;
use tokio::sync::Mutex;
use tracing::{error, trace};

use crate::{
    fcm::GoogleServices,
    route::{Route, RouteStatistics},
};

pub type AppStateType = Arc<Mutex<AppState>>;

pub struct AppState {
    pub recipient_token: String,
    pub google_services: GoogleServices,
    pub notifications: Vec<Notification>,
    pub callsign: String,
    pub vpilot_connected: bool,
    pub alarm: Option<Alarm>,
    pub stats: RouteStatistics,
    pub route: Route,
    pub alert_crashes: bool,
}

pub struct Alarm {
    pub started_at: Instant,
    pub last_notified_at: Instant,
    pub alarm_played: bool,
}

impl AppState {
    pub async fn send_notification(
        &mut self,
        message: String,
        _type: NotificationType,
    ) -> Result<()> {
        self.google_services
            .send_fcm_message(&self.recipient_token, json!({ "triggerAlarm": "true" }))
            .await
            .context("Failed to send FCM message")?;
        self.notifications.push(Notification {
            message,
            timestamp: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            _type,
        });

        let t = Instant::now();
        self.alarm = Some(Alarm {
            started_at: t,
            last_notified_at: t,
            alarm_played: false,
        });
        Ok(())
    }

    pub async fn soft_send_notification(&mut self, message: String, _type: NotificationType) {
        if self.alarm.is_none() {
            if let Err(err) = self.send_notification(message, _type).await {
                error!("Failed to send notification: {}", err);
            }
        }
    }

    pub async fn state_loop(state: AppStateType) -> Result<()> {
        loop {
            let mut state = state.lock().await;
            if let Some(alarm) = &state.alarm {
                if alarm.started_at.elapsed() > Duration::from_secs(180) {
                    state.vpilot_connected = false;
                    state.alarm = None;
                    error!("Alarm time exceeded, disconnecting from vatsim");
                } else if alarm.last_notified_at.elapsed() > Duration::from_secs(10)
                    && !alarm.alarm_played
                {
                    if let Err(err) = state
                        .google_services
                        .send_fcm_message(&state.recipient_token, json!({ "triggerAlarm": "true" }))
                        .await
                    {
                        error!("Failed to send FCM message: {}", err);
                    }
                    if let Some(alarm) = &mut state.alarm {
                        alarm.last_notified_at = Instant::now();
                    }
                }
            }

            match state.route.route_statistics().await {
                Ok(stats) => {
                    state.stats = stats;
                }
                Err(e) => error!("Failed to get route statistics: {}", e),
            };

            if state.alert_crashes {
                trace!("{:#?}", state.stats);
                let mut notifications = Vec::new();
                if state.stats.in_loop {
                    notifications.push(("In loop", NotificationType::CrashDetect));
                }

                if state.stats.stuck {
                    notifications.push(("Aircraft stuck", NotificationType::CrashDetect));
                }

                if state.stats.pilot.altitude < 29000 {
                    notifications.push(("Low altitude", NotificationType::CrashDetect));
                }

                if state.stats.pilot.ground_speed < 300 {
                    notifications.push(("Low ground speed", NotificationType::CrashDetect));
                }

                if state.stats.route_deviation > 30.0 {
                    notifications.push(("Route deviation", NotificationType::CrashDetect));
                }
                for n in notifications {
                    state.soft_send_notification(n.0.to_owned(), n.1).await
                }
            }
            drop(state);
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub enum NotificationType {
    PrivateMessage,
    RadioMessage,
    SelcalAlert,
    CrashDetect,
}

#[derive(Debug, Clone, Serialize)]
pub struct Notification {
    pub message: String,
    pub timestamp: String,
    #[serde(rename = "type")]
    pub _type: NotificationType,
}
