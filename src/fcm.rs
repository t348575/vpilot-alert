use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use eyre::{bail, Context, Result};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::{spawn, sync::Mutex, time::sleep};
use tracing::error;

#[derive(Clone, Deserialize)]
pub struct GoogleServices {
    private_key: String,
    client_email: String,
    #[serde(skip)]
    data: Arc<Mutex<ServiceData>>,
}

#[derive(Serialize)]
struct Claims {
    iss: String,
    scope: String,
    aud: String,
    exp: u64,
    iat: u64,
}

struct ServiceData {
    token_refreshed_at: Instant,
    oauth_token: TokenResponse,
}

impl Default for ServiceData {
    fn default() -> Self {
        Self {
            token_refreshed_at: Instant::now(),
            oauth_token: TokenResponse::default(),
        }
    }
}

impl GoogleServices {
    pub async fn login(&self) -> Result<()> {
        let mut token_state = self.data.lock().await;
        token_state.oauth_token = self.fetch_oauth_token().await?;
        token_state.token_refreshed_at = Instant::now();
        drop(token_state);

        let services = self.clone();
        spawn(async move {
            loop {
                let mut token_state = services.data.lock().await;
                let can_refresh = Duration::from_secs(token_state.oauth_token.expires_in)
                    - Duration::from_secs(60);
                if token_state.token_refreshed_at.elapsed() > can_refresh {
                    match services.fetch_oauth_token().await {
                        Ok(token) => {
                            token_state.oauth_token = token;
                            token_state.token_refreshed_at = Instant::now();
                            continue;
                        }
                        Err(e) => error!("Failed to refresh OAuth token: {}", e),
                    }
                }
                drop(token_state);
                sleep(can_refresh + Duration::from_secs(1)).await;
            }
        });
        Ok(())
    }

    async fn fetch_oauth_token(&self) -> Result<TokenResponse> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let claims = Claims {
            iss: self.client_email.clone(),
            scope: "https://www.googleapis.com/auth/firebase.messaging".to_string(),
            aud: "https://oauth2.googleapis.com/token".to_string(),
            exp: now + 3600,
            iat: now,
        };

        let encoding_key = EncodingKey::from_rsa_pem(self.private_key.as_bytes())
            .context("Parse google service private key")?;

        let jwt = encode(&Header::new(Algorithm::RS256), &claims, &encoding_key)
            .context("Failed to encode JWT")?;

        let client = Client::new();
        let params = [
            ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
            ("assertion", &jwt),
        ];

        let response = client
            .post("https://oauth2.googleapis.com/token")
            .form(&params)
            .send()
            .await
            .context("Send OAuth token request")?;

        let token_response = response.json().await.context("Parse token response")?;
        Ok(token_response)
    }

    async fn token(&self) -> TokenResponse {
        let token_state = self.data.lock().await;
        token_state.oauth_token.clone()
    }

    pub async fn send_fcm_message(
        &self,
        recipient_token: &str,
        data: serde_json::Value,
    ) -> Result<()> {
        let client = Client::new();
        let message = json!({
            "message": {
                "token": recipient_token,
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
                    },
                    "payload": {
                        "aps": {
                            "contentAvailable": true
                        },
                    },
                },
            }
        });

        let response = client
            .post("https://fcm.googleapis.com/v1/projects/vpilot-alert/messages:send")
            .bearer_auth(self.token().await.access_token)
            .json(&message)
            .send()
            .await
            .context("FCM message HTTP request")?;

        if !response.status().is_success() {
            bail!(
                "Failed to send FCM message: {}",
                response.text().await.context("HTTP response text")?
            );
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
struct TokenResponse {
    access_token: String,
    expires_in: u64,
}
