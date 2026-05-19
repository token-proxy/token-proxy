use serde::{Deserialize, Serialize};

/// 模型映射值对象，定义源模型到目标模型的映射关系
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelMapping {
    pub source_model: String,
    pub target_model: String,
}

impl ModelMapping {
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
    /// 返回 (新的请求体, 内容长度变化 delta)
    pub fn apply_all(&self, body: &str, requested_model: &str) -> (String, i64) {
        if let Some(mapping) = self
            .0
            .iter()
            .find(|m| m.source_model == requested_model)
        {
            mapping.apply_to_body(body)
        } else {
            (body.to_string(), 0)
        }
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
    fn test_model_mapping_apply_to_body() {
        let mapping = ModelMapping {
            source_model: "claude-sonnet-4-20250514".to_string(),
            target_model: "claude-sonnet-4-20250514-v2".to_string(),
        };
        let body = r#"{"model":"claude-sonnet-4-20250514","messages":[{"role":"user","content":"hello"}]}"#;
        let (new_body, _delta) = mapping.apply_to_body(body);
        let parsed: serde_json::Value = serde_json::from_str(&new_body).unwrap();
        assert_eq!(
            parsed["model"],
            "claude-sonnet-4-20250514-v2"
        );
    }

    #[test]
    fn test_model_mapping_apply_to_body_invalid_json() {
        let mapping = ModelMapping {
            source_model: "model-a".to_string(),
            target_model: "model-b".to_string(),
        };
        let body = "not-json";
        let (new_body, delta) = mapping.apply_to_body(body);
        assert_eq!(new_body, "not-json");
        assert_eq!(delta, 0);
    }

    #[test]
    fn test_model_mapping_collection_apply_all_match() {
        let collection = ModelMappingCollection(vec![
            ModelMapping {
                source_model: "model-a".to_string(),
                target_model: "model-b".to_string(),
            },
            ModelMapping {
                source_model: "model-x".to_string(),
                target_model: "model-y".to_string(),
            },
        ]);
        let body = r#"{"model":"model-x","messages":[]}"#;
        let (new_body, _delta) = collection.apply_all(body, "model-x");
        let parsed: serde_json::Value = serde_json::from_str(&new_body).unwrap();
        assert_eq!(parsed["model"], "model-y");
    }

    #[test]
    fn test_model_mapping_collection_apply_all_no_match() {
        let collection = ModelMappingCollection(vec![ModelMapping {
            source_model: "model-a".to_string(),
            target_model: "model-b".to_string(),
        }]);
        let body = r#"{"model":"model-z","messages":[]}"#;
        let (new_body, delta) = collection.apply_all(body, "model-z");
        assert_eq!(new_body, body);
        assert_eq!(delta, 0);
    }

    #[test]
    fn test_model_mapping_collection_empty() {
        let collection = ModelMappingCollection(vec![]);
        assert!(collection.is_empty());
        assert_eq!(collection.len(), 0);
    }
}