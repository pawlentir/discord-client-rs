use crate::BoxedResult;
use crate::rate_limit::RateLimitError;
use log::warn;
use serde_json::Value;
use std::fmt::{Display, Formatter};
use std::time::Duration;

const CLOUDFLARE_BLOCK_RETRY_AFTER: Duration = Duration::from_secs(2);

#[derive(Debug)]
pub struct DiscordApiError {
    pub status: u16,
    pub code: Option<i64>,
    pub message: Option<String>,
    pub body: String,
    pub url: String,
}

impl DiscordApiError {
    pub(crate) fn from_body(status: u16, url: &str, bytes: &[u8]) -> Self {
        let body = String::from_utf8_lossy(bytes).into_owned();
        let json = serde_json::from_slice::<Value>(bytes).ok();
        let code = json
            .as_ref()
            .and_then(|value| value.get("code"))
            .and_then(Value::as_i64);
        let message = json
            .as_ref()
            .and_then(|value| value.get("message"))
            .and_then(Value::as_str)
            .map(str::to_owned);

        Self {
            status,
            code,
            message,
            body,
            url: url.to_owned(),
        }
    }
}

impl Display for DiscordApiError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Request to {} failed with code {}: {}",
            self.url, self.status, self.body
        )
    }
}

impl std::error::Error for DiscordApiError {}

pub(crate) fn parse_error_body(bytes: &[u8], status: u16, url: &str) -> BoxedResult<Value> {
    serde_json::from_slice(bytes).map_err(|_| {
        let preview: String = String::from_utf8_lossy(bytes).chars().take(200).collect();
        format!(
            "Request to {} failed with code {} and a non-JSON body (likely a Cloudflare block \
             rather than a Discord API response): {}",
            url, status, preview
        )
        .into()
    })
}

pub(crate) fn rate_limit_from_body(bytes: &[u8], url: &str) -> RateLimitError {
    match serde_json::from_slice::<Value>(bytes) {
        Ok(json) => {
            let retry_after_secs = json["retry_after"].as_f64().unwrap_or(1.0);
            let global = json["global"].as_bool().unwrap_or(false);
            RateLimitError::new(Duration::from_secs_f64(retry_after_secs), global)
        }
        Err(_) => {
            warn!(
                "Non-JSON 429 response from {} (likely a Cloudflare edge rate limit, e.g. from a \
                 non-rotating proxy); backing off {:?} and retrying",
                url, CLOUDFLARE_BLOCK_RETRY_AFTER
            );
            RateLimitError::new(CLOUDFLARE_BLOCK_RETRY_AFTER, false)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::DiscordApiError;

    #[test]
    fn discord_api_error_extracts_structured_fields() {
        let error = DiscordApiError::from_body(
            404,
            "https://discord.com/api/v9/users/1/profile",
            br#"{"message":"Unknown User","code":10013}"#,
        );

        assert_eq!(error.status, 404);
        assert_eq!(error.code, Some(10013));
        assert_eq!(error.message.as_deref(), Some("Unknown User"));
        assert_eq!(error.body, r#"{"message":"Unknown User","code":10013}"#);
    }

    #[test]
    fn discord_api_error_preserves_non_json_body() {
        let error = DiscordApiError::from_body(503, "https://discord.com/api/v9/test", b"edge");

        assert_eq!(error.status, 503);
        assert_eq!(error.code, None);
        assert_eq!(error.message, None);
        assert_eq!(error.body, "edge");
    }
}
