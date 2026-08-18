use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Memo {
    pub id: i64,
    pub title: String,
    pub content: String,
    pub archived: bool,
    pub created_at: DateTime<Local>,
    pub updated_at: DateTime<Local>,
}
