use crate::structs::user::User;
use discord_client_macros::discord_struct;
use std::collections::HashMap;

#[discord_struct]
pub struct MessageInteraction {
    #[snowflake]
    pub id: u64,
    pub r#type: u8,
    pub name: String,
    pub user: User,
}

#[discord_struct]
pub struct MessageInteractionMetadata {
    #[snowflake]
    pub id: u64,
    pub r#type: u8,
    pub name: Option<String>,
    pub command_type: Option<u8>,
    pub ephemerality_reason: Option<u8>,
    pub user: User,
    pub authorizing_integration_owners: Option<HashMap<String, String>>,
    #[serde(default)]
    #[snowflake]
    pub original_response_message_id: Option<u64>,
    #[serde(default)]
    #[snowflake]
    pub interacted_message_id: Option<u64>,
    pub triggering_interaction_metadata: Option<Box<MessageInteractionMetadata>>,
    pub target_user: Option<User>,
    #[serde(default)]
    #[snowflake]
    pub target_message_id: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::MessageInteractionMetadata;

    const USER: &str = r#""user":{"id":"456","username":"member","discriminator":"0"}"#;

    #[test]
    fn interaction_metadata_defaults_missing_interacted_message_id() {
        let raw = format!(r#"{{"id":"123","type":2,{USER}}}"#);
        let metadata: MessageInteractionMetadata = serde_json::from_str(&raw).unwrap();

        assert_eq!(metadata.interacted_message_id, None);
    }

    #[test]
    fn interaction_metadata_parses_interacted_message_id_as_snowflake() {
        let raw = format!(r#"{{"id":"123","type":2,{USER},"interacted_message_id":"789"}}"#);
        let metadata: MessageInteractionMetadata = serde_json::from_str(&raw).unwrap();

        assert_eq!(metadata.interacted_message_id, Some(789));
    }
}
