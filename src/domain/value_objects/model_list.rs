use sea_orm::FromJsonQueryResult;
use serde::{Deserialize, Serialize};

/// 模型列表值对象，包装 Vec<String> 并通过 FromJsonQueryResult
/// 实现与 PostgreSQL JSON 列的正确序列化 / 反序列化
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default, FromJsonQueryResult)]
pub struct ModelList(pub Vec<String>);

impl ModelList {
    pub fn extend(&mut self, other: Vec<String>) {
        self.0.extend(other);
    }

    pub fn sort(&mut self) {
        self.0.sort();
    }

    pub fn dedup(&mut self) {
        self.0.dedup();
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &String> {
        self.0.iter()
    }

    pub fn inner(&self) -> &[String] {
        &self.0
    }

    pub fn contains(&self, item: &str) -> bool {
        self.0.contains(&item.to_string())
    }
}

impl From<Vec<String>> for ModelList {
    fn from(v: Vec<String>) -> Self {
        ModelList(v)
    }
}

impl From<ModelList> for Vec<String> {
    fn from(list: ModelList) -> Self {
        list.0
    }
}
