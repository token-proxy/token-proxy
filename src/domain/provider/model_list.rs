//! 模型列表值对象 — domain/provider/
//!
//! 定义 `ModelList` 包装类型，封装字符串列表并提供排序、去重、查询等操作。

use sea_orm::FromJsonQueryResult;
use serde::{Deserialize, Serialize};

/// 模型列表值对象，包装 `Vec<String>` 并提供常用集合操作
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default, FromJsonQueryResult)]
pub struct ModelList(pub Vec<String>);

impl ModelList {
    /// 合并另一个模型列表
    pub fn extend(&mut self, other: Vec<String>) {
        self.0.extend(other);
    }
    /// 排序
    pub fn sort(&mut self) {
        self.0.sort();
    }
    /// 去重
    pub fn dedup(&mut self) {
        self.0.dedup();
    }
    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    /// 返回模型数量
    pub fn len(&self) -> usize {
        self.0.len()
    }
    /// 返回迭代器
    pub fn iter(&self) -> impl Iterator<Item = &String> {
        self.0.iter()
    }
    /// 返回内部切片引用
    pub fn inner(&self) -> &[String] {
        &self.0
    }
    /// 判断是否包含指定模型
    pub fn contains(&self, item: &str) -> bool {
        self.0.contains(&item.to_string())
    }
    /// 返回第一个模型
    pub fn first(&self) -> Option<&String> {
        self.0.first()
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
