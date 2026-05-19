use crate::shared::error::AppError;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Status {
    Enabled,
    Disabled,
}

impl Status {
    pub fn is_enabled(&self) -> bool {
        matches!(self, Status::Enabled)
    }
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Status::Enabled => write!(f, "enabled"),
            Status::Disabled => write!(f, "disabled"),
        }
    }
}

impl FromStr for Status {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "enabled" | "enable" | "1" | "true" => Ok(Status::Enabled),
            "disabled" | "disable" | "0" | "false" => Ok(Status::Disabled),
            _ => Err(AppError::Validation(format!("无效的状态值: {}", s))),
        }
    }
}