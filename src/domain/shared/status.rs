use crate::shared::error::AppError;
use sea_orm::prelude::StringLen;
use sea_orm::DeriveActiveEnum;
use sea_orm::EnumIter;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::None)")]
pub enum Status {
    #[sea_orm(string_value = "enabled")]
    Enabled,
    #[sea_orm(string_value = "disabled")]
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
