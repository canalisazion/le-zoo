use serde::{Deserialize, Serialize};

// --- Système de rôles hiérarchiques ---

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    User,
    Admin,
    SuperAdmin,
    Dictateur,
    Ai,
}

impl Default for Role {
    fn default() -> Self {
        Role::User
    }
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Role::User      => write!(f, "user"),
            Role::Admin     => write!(f, "admin"),
            Role::SuperAdmin => write!(f, "super_admin"),
            Role::Dictateur => write!(f, "dictateur"),
            Role::Ai        => write!(f, "ai"),
        }
    }
}

impl Role {
    /// Dictateur gère tout le monde, SuperAdmin aussi, Admin gère les Users uniquement
    pub fn can_manage(&self, target: &Role) -> bool {
        match self {
            Role::Dictateur | Role::SuperAdmin => true,
            Role::Admin => matches!(target, Role::User),
            Role::User | Role::Ai => false,
        }
    }

    pub fn is_at_least_admin(&self) -> bool {
        matches!(self, Role::Admin | Role::SuperAdmin | Role::Dictateur)
    }

    pub fn is_dictateur(&self) -> bool {
        matches!(self, Role::Dictateur)
    }
}

// --- Structures de données ---

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct User {
    pub id: Option<String>,
    pub username: String,
    pub email: String,
    pub password: Option<String>,
    pub created_at: i64,
    #[serde(default)]
    pub role: Role,
    #[serde(default)]
    pub banned: bool,
    #[serde(default)]
    pub gender: String,
    #[serde(default)]
    pub subscribed_channels: Vec<String>,
    #[serde(default)]
    pub avatar: Option<String>,
    #[serde(default)]
    pub is_premium: bool,
    #[serde(default)]
    pub message_count: i64,
    #[serde(default)]
    pub days_active: i64,
    #[serde(default)]
    pub channels_joined: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Channel {
    pub id: Option<String>,
    pub name: String,
    pub description: String,
    pub channel_type: String,
    #[serde(default)]
    pub is_general: bool,
    #[serde(default)]
    pub is_locked: bool,
    #[serde(default)]
    pub is_gold: bool,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub topic_media: Option<String>,
    #[serde(default)]
    pub topic_text: Option<String>,
    #[serde(default)]
    pub ai_description: Option<String>,
    #[serde(default)]
    pub topic: Option<String>,
    #[serde(default)]
    pub style_color: Option<String>,
    #[serde(default)]
    pub deleted: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct ChannelWithStats {
    #[serde(flatten)]
    pub channel: Channel,
    pub message_count: i64,
    #[serde(default)]
    pub last_message_snippet: Option<String>,
    #[serde(default)]
    pub media_tag: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MessageStatus {
    Sent,
    Delivered,
    Read,
}

impl Default for MessageStatus {
    fn default() -> Self {
        MessageStatus::Sent
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Message {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub channel_id: String,
    pub author_name: String,
    pub content: String,
    pub created_at: i64,
    #[serde(default)]
    pub author_role: Role,
    #[serde(default)]
    pub author_gender: String,
    #[serde(default)]
    pub deleted: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub direct_to: Option<String>,
    #[serde(default)]
    pub reactions: Vec<Reaction>,
    #[serde(default)]
    pub status: MessageStatus,
    #[serde(default)]
    pub pinned: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Reaction {
    pub emoji: String,
    pub users: Vec<String>,
}

// --- Informations des membres (REST API) ---

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct MemberInfo {
    pub username: String,
    pub role: Role,
    pub online: bool,
    #[serde(default)]
    pub gender: String,
    #[serde(default)]
    pub avatar: Option<String>,
    #[serde(default)]
    pub badges: Vec<String>,
}

// --- Messages WebSocket (protocole JSON) ---

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsClientMsg {
    Chat { content: String },
    DirectMessage { to: String, content: String },
    MarkAsRead { message_id: String },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsServerMsg {
    Chat {
        author: String,
        content: String,
        role: Role,
        #[serde(default)]
        gender: String,
    },
    DirectMessage {
        from: String,
        content: String,
        role: Role,
        #[serde(default)]
        gender: String,
        id: String,
    },
    Presence {
        online: Vec<String>,
    },
    Ack {
        ok: bool,
        #[serde(default)]
        message_id: Option<String>,
    },
    MessageStatusUpdated {
        message_id: String,
        status: MessageStatus,
    },
    MessageDeleted {
        id: String,
    },
}

// --- Système d'articles magazine ---

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum ArticleCategory {
    Dossier,
    Insolite,
    FaitsDivers,
    Culture,
    Formation,
    Critique,
    Billet,
}

impl std::fmt::Display for ArticleCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            ArticleCategory::Dossier    => "dossier",
            ArticleCategory::Insolite   => "insolite",
            ArticleCategory::FaitsDivers => "faits-divers",
            ArticleCategory::Culture    => "culture",
            ArticleCategory::Formation  => "formation",
            ArticleCategory::Critique   => "critique",
            ArticleCategory::Billet     => "billet",
        };
        write!(f, "{}", s)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Article {
    #[serde(default)]
    pub id: Option<String>,
    pub slug: String,
    pub title: String,
    pub content: String,
    pub excerpt: String,
    pub category: ArticleCategory,
    pub author_name: String,
    #[serde(default)]
    pub cover_image: Option<String>,
    pub published: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct ArticleSummary {
    #[serde(default)]
    pub id: Option<String>,
    pub slug: String,
    pub title: String,
    pub excerpt: String,
    pub category: ArticleCategory,
    pub author_name: String,
    #[serde(default)]
    pub cover_image: Option<String>,
    pub created_at: i64,
}

