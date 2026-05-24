use crate::domain::value_objects::model_mapping::{
    MatchType, ModelMapping, DEFAULT_MODEL_SENTINEL, UNMATCHED_MODEL_SENTINEL,
};

/// 应用模型映射到请求体的纯函数
///
/// 查找与请求模型匹配的映射，优先级：
/// 1. 精确匹配
/// 2. 前缀匹配
/// 3. `__unmatched__` 规则
/// 4. 如果均未匹配，返回 (原始请求体, 0, None)
///
/// # 参数
/// - `request_body`: 原始请求体 JSON 字符串
/// - `mappings`: 模型映射列表
/// - `requested_model`: 请求中使用的模型名称
///
/// # 返回
/// - `(String, i64, Option<String>)`: (新的请求体, 内容长度变化 delta, 映射后的目标模型名)
pub fn apply_model_mappings(
    request_body: &str,
    mappings: &[ModelMapping],
    requested_model: &str,
) -> (String, i64, Option<String>) {
    if let Some(mapping) = find_matching_mapping(mappings, requested_model) {
        let (new_body, delta) = mapping.apply_to_body(request_body);
        (new_body, delta, Some(mapping.target_model.clone()))
    } else {
        (request_body.to_string(), 0, None)
    }
}

/// 在映射列表中查找与请求模型匹配的映射
///
/// 优先级规则：
/// 1. 精确匹配（source_model == requested_model，且 match_type == Exact）
/// 2. 前缀匹配（requested_model 以 source_model 开头，且 match_type == Prefix）
/// 3. `__unmatched__` 规则（source_model == UNMATCHED_MODEL_SENTINEL）
pub fn find_matching_mapping<'a>(
    mappings: &'a [ModelMapping],
    requested_model: &str,
) -> Option<&'a ModelMapping> {
    // 1. 精确匹配
    if let Some(m) = mappings
        .iter()
        .find(|m| m.match_type == MatchType::Exact && m.source_model == requested_model)
    {
        return Some(m);
    }
    // 2. 前缀匹配
    if let Some(m) = mappings
        .iter()
        .find(|m| m.match_type == MatchType::Prefix && requested_model.starts_with(&m.source_model))
    {
        return Some(m);
    }
    // 3. __unmatched__ 规则
    if let Some(m) = mappings
        .iter()
        .find(|m| m.source_model == UNMATCHED_MODEL_SENTINEL)
    {
        return Some(m);
    }
    None
}

/// 确定最终使用的模型名称
///
/// 优先级：
/// 1. 映射后的模型名（如果匹配到映射规则）
/// 2. Provider 的默认模型（如果配置了）
/// 3. 原始请求的模型名（兜底）
pub fn resolve_final_model(
    mapped_model: Option<String>,
    default_model: Option<&str>,
    original_model: &str,
) -> String {
    match mapped_model.as_deref() {
        Some(DEFAULT_MODEL_SENTINEL) => default_model
            .map(|s| s.to_string())
            .unwrap_or_else(|| original_model.to_string()),
        Some(model) => model.to_string(),
        None => default_model
            .map(|s| s.to_string())
            .unwrap_or_else(|| original_model.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_matching_mapping_exact() {
        let mappings = vec![
            ModelMapping::new_exact("gpt-4".to_string(), "gpt-4-turbo".to_string()),
            ModelMapping::new_prefix("claude-".to_string(), "claude-3-opus".to_string()),
        ];
        let found = find_matching_mapping(&mappings, "gpt-4");
        assert!(found.is_some());
        assert_eq!(found.unwrap().target_model, "gpt-4-turbo");
    }

    #[test]
    fn test_find_matching_mapping_prefix() {
        let mappings = vec![
            ModelMapping::new_exact("gpt-4".to_string(), "gpt-4-turbo".to_string()),
            ModelMapping::new_prefix(
                "claude-sonnet-".to_string(),
                "claude-sonnet-4-20250514".to_string(),
            ),
        ];
        let found = find_matching_mapping(&mappings, "claude-sonnet-4-20250101");
        assert!(found.is_some());
        assert_eq!(found.unwrap().target_model, "claude-sonnet-4-20250514");
    }

    #[test]
    fn test_find_matching_mapping_unmatched() {
        let mappings = vec![
            ModelMapping::new_exact("gpt-4".to_string(), "gpt-4-turbo".to_string()),
            ModelMapping::new_exact(
                UNMATCHED_MODEL_SENTINEL.to_string(),
                "claude-sonnet-4-20250514".to_string(),
            ),
        ];
        let found = find_matching_mapping(&mappings, "unknown-model");
        assert!(found.is_some());
        assert_eq!(found.unwrap().target_model, "claude-sonnet-4-20250514");
    }

    #[test]
    fn test_find_matching_mapping_no_match() {
        let mappings = vec![ModelMapping::new_exact(
            "gpt-4".to_string(),
            "gpt-4-turbo".to_string(),
        )];
        let found = find_matching_mapping(&mappings, "unknown-model");
        assert!(found.is_none());
    }

    #[test]
    fn test_apply_model_mappings_with_match() {
        let mappings = vec![ModelMapping::new_exact(
            "claude-sonnet-4-20250514".to_string(),
            "claude-sonnet-4-20250514-v2".to_string(),
        )];
        let body =
            r#"{"model":"claude-sonnet-4-20250514","messages":[{"role":"user","content":"hi"}]}"#;
        let (new_body, _delta, mapped) =
            apply_model_mappings(body, &mappings, "claude-sonnet-4-20250514");
        let parsed: serde_json::Value = serde_json::from_str(&new_body).unwrap();
        assert_eq!(parsed["model"], "claude-sonnet-4-20250514-v2");
        assert_eq!(mapped, Some("claude-sonnet-4-20250514-v2".to_string()));
    }

    #[test]
    fn test_apply_model_mappings_no_match() {
        let mappings = vec![ModelMapping::new_exact(
            "model-a".to_string(),
            "model-b".to_string(),
        )];
        let body = r#"{"model":"model-c","messages":[]}"#;
        let (new_body, delta, mapped) = apply_model_mappings(body, &mappings, "model-c");
        assert_eq!(new_body, body);
        assert_eq!(delta, 0);
        assert_eq!(mapped, None);
    }

    #[test]
    fn test_apply_model_mappings_empty_mappings() {
        let mappings = vec![];
        let body = r#"{"model":"test-model","messages":[]}"#;
        let (new_body, delta, mapped) = apply_model_mappings(body, &mappings, "test-model");
        assert_eq!(new_body, body);
        assert_eq!(delta, 0);
        assert_eq!(mapped, None);
    }

    #[test]
    fn test_resolve_final_model_mapped_wins() {
        let result = resolve_final_model(
            Some("mapped-model".to_string()),
            Some("default"),
            "original",
        );
        assert_eq!(result, "mapped-model");
    }

    #[test]
    fn test_resolve_final_model_default_sentinel() {
        let result = resolve_final_model(
            Some(DEFAULT_MODEL_SENTINEL.to_string()),
            Some("default-model"),
            "original",
        );
        assert_eq!(result, "default-model");
    }

    #[test]
    fn test_resolve_final_model_default_fallback() {
        let result = resolve_final_model(None, Some("default-model"), "original");
        assert_eq!(result, "default-model");
    }

    #[test]
    fn test_resolve_final_model_original_fallback() {
        let result: String = resolve_final_model(None, None, "original-model");
        assert_eq!(result, "original-model");
    }
}
