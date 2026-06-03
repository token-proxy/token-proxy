use crate::shared::error::AppError;
use sea_orm::prelude::StringLen;
use sea_orm::DeriveActiveEnum;
use sea_orm::EnumIter;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::None)")]
pub enum AccessPointType {
    #[sea_orm(string_value = "anthropic")]
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
