use crate::domain::value_objects::model_mapping::ModelMapping;

/// 应用模型映射到请求体的纯函数
///
/// 查找与请求模型匹配的映射，替换 JSON 中的 model 字段为目标模型。
/// 如果找不到匹配映射，则返回原始请求体和 0 长度变化。
///
/// # 参数
/// - `request_body`: 原始请求体 JSON 字符串
/// - `mappings`: 模型映射列表
/// - `requested_model`: 请求中使用的模型名称
///
/// # 返回
/// - `(String, i64)`: (新的请求体, 内容长度变化 delta)
pub fn apply_model_mappings(
    request_body: &str,
    mappings: &[ModelMapping],
    requested_model: &str,
) -> (String, i64) {
    if let Some(mapping) = mappings.iter().find(|m| m.source_model == requested_model) {
        mapping.apply_to_body(request_body)
    } else {
        (request_body.to_string(), 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_model_mappings_with_match() {
        let mappings = vec![ModelMapping {
            source_model: "claude-sonnet-4-20250514".to_string(),
            target_model: "claude-sonnet-4-20250514-v2".to_string(),
        }];
        let body =
            r#"{"model":"claude-sonnet-4-20250514","messages":[{"role":"user","content":"hi"}]}"#;
        let (new_body, _delta) = apply_model_mappings(body, &mappings, "claude-sonnet-4-20250514");
        let parsed: serde_json::Value = serde_json::from_str(&new_body).unwrap();
        assert_eq!(parsed["model"], "claude-sonnet-4-20250514-v2");
    }

    #[test]
    fn test_apply_model_mappings_no_match() {
        let mappings = vec![ModelMapping {
            source_model: "model-a".to_string(),
            target_model: "model-b".to_string(),
        }];
        let body = r#"{"model":"model-c","messages":[]}"#;
        let (new_body, delta) = apply_model_mappings(body, &mappings, "model-c");
        assert_eq!(new_body, body);
        assert_eq!(delta, 0);
    }

    #[test]
    fn test_apply_model_mappings_empty_mappings() {
        let mappings = vec![];
        let body = r#"{"model":"test-model","messages":[]}"#;
        let (new_body, delta) = apply_model_mappings(body, &mappings, "test-model");
        assert_eq!(new_body, body);
        assert_eq!(delta, 0);
    }
}
