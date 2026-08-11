use discord_client_structs::deserializer::deserialize_string_to_u64;
use discord_client_structs::structs::channel::status::ChannelStatus;
use discord_client_structs::structs::message::Message;
use discord_client_structs::structs::user::Member;
use discord_client_structs::structs::user::presence::Presence;
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize, Clone)]
pub struct ChannelStatusesEvent {
    #[serde(deserialize_with = "deserialize_string_to_u64")]
    pub guild_id: u64,
    pub channels: Vec<ChannelStatus>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LastMessagesEvent {
    #[serde(deserialize_with = "deserialize_string_to_u64")]
    pub guild_id: u64,
    pub messages: Vec<Message>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct GuildMembersChunkEvent {
    #[serde(deserialize_with = "deserialize_string_to_u64")]
    pub guild_id: u64,
    pub members: Vec<Member>,
    pub chunk_index: u64,
    pub chunk_count: u64,
    pub not_found: Option<Vec<String>>,
    pub presences: Option<Vec<Presence>>,
    pub nonce: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct GuildMemberListUpdateEvent {
    #[serde(default)]
    pub id: String,
    #[serde(deserialize_with = "deserialize_string_to_u64")]
    pub guild_id: u64,
    #[serde(default)]
    pub member_count: u64,
    #[serde(default)]
    pub online_count: u64,
    #[serde(default)]
    pub groups: Vec<GuildMemberListGroup>,
    #[serde(default)]
    pub ops: Vec<GuildMemberListOperation>,
}

impl GuildMemberListUpdateEvent {
    pub fn synced_members(&self) -> impl Iterator<Item = &Member> {
        self.ops
            .iter()
            .filter(|operation| operation.op == "SYNC")
            .flat_map(|operation| operation.items.iter())
            .filter_map(|item| item.member.as_ref())
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct GuildMemberListGroup {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub count: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct GuildMemberListOperation {
    #[serde(default)]
    pub op: String,
    #[serde(default)]
    pub range: Option<[u64; 2]>,
    #[serde(default)]
    pub items: Vec<GuildMemberListItem>,
    #[serde(default)]
    pub index: Option<u64>,
    #[serde(default)]
    pub item: Option<Value>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct GuildMemberListItem {
    #[serde(default)]
    pub member: Option<Member>,
    #[serde(default)]
    pub group: Option<GuildMemberListGroup>,
}

#[cfg(test)]
mod tests {
    use super::GuildMemberListUpdateEvent;
    use serde_json::json;

    #[test]
    fn member_list_update_parses_sync_members_and_tolerates_other_operations() {
        let update: GuildMemberListUpdateEvent = serde_json::from_value(json!({
            "id": "everyone",
            "guild_id": "123",
            "member_count": 2,
            "online_count": 1,
            "groups": [
                {"id": "online", "count": 1},
                {"id": "offline", "count": 1}
            ],
            "ops": [
                {
                    "op": "SYNC",
                    "range": [0, 99],
                    "items": [
                        {"group": {"id": "online", "count": 1}},
                        {
                            "member": {
                                "user": {
                                    "id": "456",
                                    "username": "member",
                                    "discriminator": "0"
                                },
                                "roles": [],
                                "joined_at": "2024-01-01T00:00:00Z",
                                "deaf": false,
                                "mute": false,
                                "flags": 0
                            }
                        }
                    ]
                },
                {"op": "UPDATE", "index": 0, "item": {"future": true}},
                {"op": "INSERT", "index": 1, "item": {"future": true}},
                {"op": "DELETE", "index": 2},
                {"op": "INVALIDATE", "range": [100, 199]},
                {"op": "SOMETHING_NEW", "unknown": {"nested": true}}
            ]
        }))
        .unwrap();

        assert_eq!(update.guild_id, 123);
        assert_eq!(update.id, "everyone");
        assert_eq!(update.member_count, 2);
        assert_eq!(update.ops.len(), 6);
        assert_eq!(update.ops[0].range, Some([0, 99]));
        assert_eq!(update.ops[1].index, Some(0));

        let members = update.synced_members().collect::<Vec<_>>();
        assert_eq!(members.len(), 1);
        assert_eq!(members[0].user.as_ref().unwrap().id, 456);
    }
}
