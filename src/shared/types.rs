use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub type Timestamp = DateTime<Utc>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResult<T> {
    pub items: Vec<T>,
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PaginationParams {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

impl PaginationParams {
    pub fn limit(&self) -> u64 {
        self.page_size.unwrap_or(20).min(100)
    }

    pub fn offset(&self) -> u64 {
        let page = self.page.unwrap_or(1).max(1);
        (page - 1) * self.limit()
    }
}