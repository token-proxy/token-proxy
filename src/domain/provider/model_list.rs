use sea_orm::FromJsonQueryResult;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default, FromJsonQueryResult)]
pub struct ModelList(pub Vec<String>);

impl ModelList {
    pub fn extend(&mut self, other: Vec<String>) { self.0.extend(other); }
    pub fn sort(&mut self) { self.0.sort(); }
    pub fn dedup(&mut self) { self.0.dedup(); }
    pub fn is_empty(&self) -> bool { self.0.is_empty() }
    pub fn len(&self) -> usize { self.0.len() }
    pub fn iter(&self) -> impl Iterator<Item = &String> { self.0.iter() }
    pub fn inner(&self) -> &[String] { &self.0 }
    pub fn contains(&self, item: &str) -> bool { self.0.contains(&item.to_string()) }
    pub fn first(&self) -> Option<&String> { self.0.first() }
}

impl From<Vec<String>> for ModelList {
    fn from(v: Vec<String>) -> Self { ModelList(v) }
}

impl From<ModelList> for Vec<String> {
    fn from(list: ModelList) -> Self { list.0 }
}
