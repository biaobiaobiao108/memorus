use chrono::{DateTime, Local};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Memo {
    pub id: i64,
    pub title: String,
    pub content: String,
    pub created_at: DateTime<Local>,
    pub updated_at: DateTime<Local>,
}
