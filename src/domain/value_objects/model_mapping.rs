use serde::{Deserialize, Serialize};

/// 未匹配的模型哨兵常量：当没有精确/前缀匹配时，使用此规则
pub const UNMATCHED_MODEL_SENTINEL: &str = "__unmatched__";
pub const DEFAULT_MODEL_SENTINEL: &str = "__default_model__";

/// Claude 模型族前缀常量
pub const CLAUDE_OPUS_PREFIX: &str = "claude-opus-";
pub const CLAUDE_SONNET_PREFIX: &str = "claude-sonnet-";
pub const CLAUDE_HAIKU_PREFIX: &str = "claude-haiku-";

/// 模型映射匹配方式
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MatchType {
    /// 精确匹配（默认）
    #[default]
    Exact,
    /// 前缀匹配
    Prefix,
}

impl MatchType {
    pub fn as_str(&self) -> &str {
        match self {
            MatchType::Exact => "exact",
            MatchType::Prefix => "prefix",
        }
    }

    pub fn from_str_value(s: &str) -> Option<Self> {
        match s {
            "exact" => Some(MatchType::Exact),
            "prefix" => Some(MatchType::Prefix),
            _ => None,
        }
    }
}

/// 模型映射值对象，定义源模型到目标模型的映射关系
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelMapping {
    pub source_model: String,
    pub target_model: String,
    #[serde(default)]
    pub match_type: MatchType,
}

impl ModelMapping {
    /// 创建精确匹配的映射
    pub fn new_exact(source_model: String, target_model: String) -> Self {
        ModelMapping {
            source_model,
            target_model,
            match_type: MatchType::Exact,
        }
    }

    /// 创建前缀匹配的映射
    pub fn new_prefix(source_model: String, target_model: String) -> Self {
        ModelMapping {
            source_model,
            target_model,
            match_type: MatchType::Prefix,
        }
    }

    /// 判断此映射是否能匹配指定的模型名
    pub fn matches(&self, model: &str) -> bool {
        match self.match_type {
            MatchType::Exact => self.source_model == model,
            MatchType::Prefix => model.starts_with(&self.source_model),
        }
    }

    /// 将当前映射应用到请求体 JSON 中，替换 model 字段为目标模型
    /// 返回 (新的请求体, 内容长度变化 delta)
    pub fn apply_to_body(&self, body: &str) -> (String, i64) {
        match serde_json::from_str::<serde_json::Value>(body) {
            Ok(mut value) => {
                let original_len = body.len() as i64;
                if let Some(obj) = value.as_object_mut() {
                    obj.insert(
                        "model".to_string(),
                        serde_json::Value::String(self.target_model.clone()),
                    );
                }
                let new_body = serde_json::to_string(&value).unwrap_or_else(|_| body.to_string());
                let new_len = new_body.len() as i64;
                (new_body, new_len - original_len)
            }
            Err(_) => (body.to_string(), 0),
        }
    }
}

/// 模型映射集合，包装 Vec<ModelMapping> 并提供批量应用方法
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelMappingCollection(pub Vec<ModelMapping>);

impl ModelMappingCollection {
    /// 根据请求中的 model 名称查找匹配的映射并应用
    /// 优先级：精确匹配 > 前缀匹配 > __unmatched__ 规则
    /// 返回 (新的请求体, 内容长度变化 delta, 匹配到的目标模型 Option)
    pub fn apply_all(&self, body: &str, requested_model: &str) -> (String, i64) {
        if let Some(mapping) = self.find_mapping(requested_model) {
            mapping.apply_to_body(body)
        } else {
            (body.to_string(), 0)
        }
    }

    /// 查找与请求模型匹配的映射
    /// 优先级：精确匹配 > 前缀匹配 > __unmatched__ 规则
    pub fn find_mapping(&self, requested_model: &str) -> Option<&ModelMapping> {
        // 1. 精确匹配
        if let Some(m) = self
            .0
            .iter()
            .find(|m| m.match_type == MatchType::Exact && m.source_model == requested_model)
        {
            return Some(m);
        }
        // 2. 前缀匹配
        if let Some(m) = self.0.iter().find(|m| {
            m.match_type == MatchType::Prefix && requested_model.starts_with(&m.source_model)
        }) {
            return Some(m);
        }
        // 3. __unmatched__ 规则
        if let Some(m) = self
            .0
            .iter()
            .find(|m| m.source_model == UNMATCHED_MODEL_SENTINEL)
        {
            return Some(m);
        }
        None
    }

    /// 查找与请求模型匹配的映射，返回映射后的目标模型名称
    /// 如果未找到映射则返回 None
    pub fn map_model(&self, requested_model: &str) -> Option<String> {
        self.find_mapping(requested_model)
            .map(|m| m.target_model.clone())
    }

    /// 返回内部映射列表的引用
    pub fn inner(&self) -> &[ModelMapping] {
        &self.0
    }

    /// 判断集合是否为空
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// 返回映射数量
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

impl From<Vec<ModelMapping>> for ModelMappingCollection {
    fn from(mappings: Vec<ModelMapping>) -> Self {
        ModelMappingCollection(mappings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_match_type_default() {
        assert_eq!(MatchType::default(), MatchType::Exact);
    }

    #[test]
    fn test_match_type_roundtrip() {
        assert_eq!(MatchType::from_str_value("exact"), Some(MatchType::Exact));
        assert_eq!(MatchType::from_str_value("prefix"), Some(MatchType::Prefix));
        assert_eq!(MatchType::from_str_value("unknown"), None);
        assert_eq!(MatchType::Exact.as_str(), "exact");
        assert_eq!(MatchType::Prefix.as_str(), "prefix");
    }

    #[test]
    fn test_model_mapping_matches_exact() {
        let mapping = ModelMapping::new_exact("gpt-4".to_string(), "gpt-4-turbo".to_string());
        assert!(mapping.matches("gpt-4"));
        assert!(!mapping.matches("gpt-4-turbo"));
        assert!(!mapping.matches("gpt-3.5"));
    }

    #[test]
    fn test_model_mapping_matches_prefix() {
        let mapping = ModelMapping::new_prefix(
            "claude-sonnet-".to_string(),
            "claude-sonnet-4-20250514".to_string(),
        );
        assert!(mapping.matches("claude-sonnet-4-20250514"));
        assert!(mapping.matches("claude-sonnet-4-20240101"));
        assert!(!mapping.matches("claude-opus-4-20250514"));
        assert!(!mapping.matches("gpt-4"));
    }

    #[test]
    fn test_model_mapping_apply_to_body() {
        let mapping = ModelMapping {
            source_model: "claude-sonnet-4-20250514".to_string(),
            target_model: "claude-sonnet-4-20250514-v2".to_string(),
            match_type: MatchType::Exact,
        };
        let body = r#"{"model":"claude-sonnet-4-20250514","messages":[{"role":"user","content":"hello"}]}"#;
        let (new_body, _delta) = mapping.apply_to_body(body);
        let parsed: serde_json::Value = serde_json::from_str(&new_body).unwrap();
        assert_eq!(parsed["model"], "claude-sonnet-4-20250514-v2");
    }

    #[test]
    fn test_model_mapping_apply_to_body_invalid_json() {
        let mapping = ModelMapping {
            source_model: "model-a".to_string(),
            target_model: "model-b".to_string(),
            match_type: MatchType::Exact,
        };
        let body = "not-json";
        let (new_body, delta) = mapping.apply_to_body(body);
        assert_eq!(new_body, "not-json");
        assert_eq!(delta, 0);
    }

    #[test]
    fn test_collection_find_mapping_exact_first() {
        let collection = ModelMappingCollection(vec![
            ModelMapping::new_exact("gpt-4".to_string(), "gpt-4-turbo".to_string()),
            ModelMapping::new_prefix("gpt-".to_string(), "gpt-4".to_string()),
        ]);
        // 精确匹配优先于前缀匹配
        let found = collection.find_mapping("gpt-4");
        assert!(found.is_some());
        assert_eq!(found.unwrap().match_type, MatchType::Exact);
        assert_eq!(found.unwrap().target_model, "gpt-4-turbo");
    }

    #[test]
    fn test_collection_find_mapping_prefix() {
        let collection = ModelMappingCollection(vec![
            ModelMapping::new_prefix(
                "claude-sonnet-".to_string(),
                "claude-sonnet-4-20250514".to_string(),
            ),
            ModelMapping::new_prefix("claude-".to_string(), "claude-3-opus".to_string()),
        ]);
        // 最具体的前缀应该先声明，这里检查匹配到第一个前缀
        let found = collection.find_mapping("claude-sonnet-4-20250101");
        assert!(found.is_some());
        assert_eq!(found.unwrap().match_type, MatchType::Prefix);
    }

    #[test]
    fn test_collection_find_mapping_unmatched() {
        let collection = ModelMappingCollection(vec![
            ModelMapping::new_exact("gpt-4".to_string(), "gpt-4-turbo".to_string()),
            ModelMapping::new_exact(
                UNMATCHED_MODEL_SENTINEL.to_string(),
                "claude-sonnet-4-20250514".to_string(),
            ),
        ]);
        // 未匹配模型应使用 __unmatched__ 规则
        let found = collection.find_mapping("unknown-model");
        assert!(found.is_some());
        assert_eq!(found.unwrap().target_model, "claude-sonnet-4-20250514");
    }

    #[test]
    fn test_collection_find_mapping_no_match() {
        let collection = ModelMappingCollection(vec![ModelMapping::new_exact(
            "gpt-4".to_string(),
            "gpt-4-turbo".to_string(),
        )]);
        let found = collection.find_mapping("unknown-model");
        assert!(found.is_none());
    }

    #[test]
    fn test_collection_map_model() {
        let collection = ModelMappingCollection(vec![ModelMapping::new_exact(
            "model-a".to_string(),
            "model-b".to_string(),
        )]);
        assert_eq!(collection.map_model("model-a"), Some("model-b".to_string()));
        assert_eq!(collection.map_model("model-c"), None);
    }

    #[test]
    fn test_collection_apply_all_match() {
        let collection = ModelMappingCollection(vec![
            ModelMapping::new_exact("model-a".to_string(), "model-b".to_string()),
            ModelMapping::new_exact("model-x".to_string(), "model-y".to_string()),
        ]);
        let body = r#"{"model":"model-x","messages":[]}"#;
        let (new_body, _delta) = collection.apply_all(body, "model-x");
        let parsed: serde_json::Value = serde_json::from_str(&new_body).unwrap();
        assert_eq!(parsed["model"], "model-y");
    }

    #[test]
    fn test_collection_apply_all_no_match() {
        let collection = ModelMappingCollection(vec![ModelMapping::new_exact(
            "model-a".to_string(),
            "model-b".to_string(),
        )]);
        let body = r#"{"model":"model-z","messages":[]}"#;
        let (new_body, delta) = collection.apply_all(body, "model-z");
        assert_eq!(new_body, body);
        assert_eq!(delta, 0);
    }

    #[test]
    fn test_collection_empty() {
        let collection = ModelMappingCollection(vec![]);
        assert!(collection.is_empty());
        assert_eq!(collection.len(), 0);
        assert!(collection.find_mapping("any").is_none());
    }

    #[test]
    fn test_constants() {
        assert_eq!(UNMATCHED_MODEL_SENTINEL, "__unmatched__");
        assert_eq!(CLAUDE_OPUS_PREFIX, "claude-opus-");
        assert_eq!(CLAUDE_SONNET_PREFIX, "claude-sonnet-");
        assert_eq!(CLAUDE_HAIKU_PREFIX, "claude-haiku-");
    }
}
