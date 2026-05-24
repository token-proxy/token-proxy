use crate::shared::error::AppError;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AccessPointType {
    Anthropic,
}

impl AccessPointType {
    pub fn all_variants() -> Vec<AccessPointType> {
        vec![AccessPointType::Anthropic]
    }
}

impl fmt::Display for AccessPointType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AccessPointType::Anthropic => write!(f, "anthropic"),
        }
    }
}

impl FromStr for AccessPointType {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "anthropic" => Ok(AccessPointType::Anthropic),
            _ => Err(AppError::Validation(format!("不支持的接入点类型: {}", s))),
        }
    }
}
