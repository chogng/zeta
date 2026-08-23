use crate::oauth::ChatGptError;
use base64::Engine;
use serde::Deserialize;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::time::Duration;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;
use zeroize::Zeroize;

pub(crate) const REFRESH_MARGIN: Duration = Duration::from_secs(5 * 60);

#[derive(Deserialize, Serialize)]
pub(crate) struct TokenCredential {
    pub(crate) id_token: String,
    pub(crate) access_token: String,
    pub(crate) refresh_token: String,
    pub(crate) expires_at: Option<u64>,
    pub(crate) email: Option<String>,
    pub(crate) plan: Option<String>,
    pub(crate) user_id: Option<String>,
    pub(crate) account_id: Option<String>,
    pub(crate) is_fedramp: bool,
    pub(crate) credential_revision: u64,
}

impl TokenCredential {
    pub(crate) fn from_tokens(
        mut tokens: TokenResponse,
        credential_revision: u64,
    ) -> Result<Self, ChatGptError> {
        if tokens.id_token.trim().is_empty() || tokens.access_token.trim().is_empty() {
            return Err(ChatGptError::new(
                "OpenAI returned an incomplete token response",
            ));
        }
        let claims: IdentityClaims = decode_jwt_payload(&tokens.id_token)?;
        let access_claims: ExpirationClaims = decode_jwt_payload(&tokens.access_token)?;
        let auth = claims.auth.unwrap_or_default();
        Ok(Self {
            id_token: std::mem::take(&mut tokens.id_token),
            access_token: std::mem::take(&mut tokens.access_token),
            refresh_token: std::mem::take(&mut tokens.refresh_token),
            expires_at: access_claims
                .exp
                .and_then(|value| u64::try_from(value).ok()),
            email: claims
                .email
                .or_else(|| claims.profile.and_then(|value| value.email)),
            plan: auth.chatgpt_plan_type,
            user_id: auth.chatgpt_user_id.or(auth.user_id),
            account_id: auth.chatgpt_account_id,
            is_fedramp: auth.chatgpt_account_is_fedramp,
            credential_revision,
        })
    }

    pub(crate) fn apply_refresh(
        &mut self,
        mut response: RefreshResponse,
    ) -> Result<(), ChatGptError> {
        let id_token = response
            .id_token
            .take()
            .unwrap_or_else(|| self.id_token.clone());
        let access_token = response
            .access_token
            .take()
            .unwrap_or_else(|| self.access_token.clone());
        let refresh_token = response
            .refresh_token
            .take()
            .unwrap_or_else(|| self.refresh_token.clone());
        let refreshed = Self::from_tokens(
            TokenResponse {
                id_token,
                access_token,
                refresh_token,
            },
            self.credential_revision.saturating_add(1),
        )?;
        *self = refreshed;
        Ok(())
    }

    pub(crate) fn needs_refresh(&self) -> bool {
        self.expires_at.is_some_and(|expires_at| {
            expires_at <= now_epoch_seconds().saturating_add(REFRESH_MARGIN.as_secs())
        })
    }

    pub(crate) fn is_usable(&self) -> bool {
        !self.access_token.trim().is_empty()
            && (!self.needs_refresh() || !self.refresh_token.trim().is_empty())
    }
}

impl Drop for TokenCredential {
    fn drop(&mut self) {
        self.id_token.zeroize();
        self.access_token.zeroize();
        self.refresh_token.zeroize();
        self.email.zeroize();
        self.user_id.zeroize();
        self.account_id.zeroize();
    }
}

#[derive(Deserialize)]
pub(crate) struct TokenResponse {
    pub(crate) id_token: String,
    pub(crate) access_token: String,
    pub(crate) refresh_token: String,
}

impl Drop for TokenResponse {
    fn drop(&mut self) {
        self.id_token.zeroize();
        self.access_token.zeroize();
        self.refresh_token.zeroize();
    }
}

#[derive(Deserialize, Serialize)]
pub(crate) struct RefreshRequest<'a> {
    pub(crate) client_id: &'a str,
    pub(crate) grant_type: &'static str,
    pub(crate) refresh_token: &'a str,
}

#[derive(Deserialize)]
pub(crate) struct RefreshResponse {
    #[serde(default)]
    pub(crate) id_token: Option<String>,
    #[serde(default)]
    pub(crate) access_token: Option<String>,
    #[serde(default)]
    pub(crate) refresh_token: Option<String>,
}

impl Drop for RefreshResponse {
    fn drop(&mut self) {
        self.id_token.zeroize();
        self.access_token.zeroize();
        self.refresh_token.zeroize();
    }
}

#[derive(Deserialize)]
struct IdentityClaims {
    #[serde(default)]
    email: Option<String>,
    #[serde(rename = "https://api.openai.com/profile", default)]
    profile: Option<ProfileClaims>,
    #[serde(rename = "https://api.openai.com/auth", default)]
    auth: Option<AuthClaims>,
}

#[derive(Deserialize)]
struct ProfileClaims {
    #[serde(default)]
    email: Option<String>,
}

#[derive(Default, Deserialize)]
struct AuthClaims {
    #[serde(default)]
    chatgpt_plan_type: Option<String>,
    #[serde(default)]
    chatgpt_user_id: Option<String>,
    #[serde(default)]
    user_id: Option<String>,
    #[serde(default)]
    chatgpt_account_id: Option<String>,
    #[serde(default)]
    chatgpt_account_is_fedramp: bool,
}

#[derive(Deserialize)]
struct ExpirationClaims {
    #[serde(default)]
    exp: Option<i64>,
}

fn decode_jwt_payload<T: DeserializeOwned>(jwt: &str) -> Result<T, ChatGptError> {
    let mut parts = jwt.split('.');
    let payload = match (parts.next(), parts.next(), parts.next(), parts.next()) {
        (Some(header), Some(payload), Some(signature), None)
            if !header.is_empty() && !payload.is_empty() && !signature.is_empty() =>
        {
            payload
        }
        _ => return Err(ChatGptError::new("OpenAI returned an invalid token")),
    };
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload)
        .map_err(|_| ChatGptError::new("OpenAI returned an invalid token"))?;
    serde_json::from_slice(&bytes)
        .map_err(|_| ChatGptError::new("OpenAI returned an invalid token"))
}

pub(crate) fn now_epoch_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
