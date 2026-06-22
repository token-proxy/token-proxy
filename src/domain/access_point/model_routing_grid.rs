//! 模型路由网格 — domain/access_point/
//!
//! 定义 `ModelRoutingGrid`（二维表：source_model × provider_id → target_model）
//! 和 `ModelRoutingRow`（单行记录）。
//!
//! 匹配优先级：精确匹配 > 前缀匹配 > `__unmatched__` 兜底 > 返回原始模型。

use sea_orm::FromJsonQueryResult;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use super::is_prefix_source_model;

/// 未匹配模型占位符常量
pub const UNMATCHED_MODEL: &str = "__unmatched__";

/// 模型路由网格值对象
///
/// 二维表结构：source_model × provider_id → target_model。
/// 匹配优先级：精确匹配 > 前缀匹配 > `__unmatched__` 兜底 > 返回原始模型。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default, FromJsonQueryResult)]
pub struct ModelRoutingGrid {
    pub provider_ids: Vec<Uuid>,
    pub rows: Vec<ModelRoutingRow>,
}

/// 路由网格中的一行，定义单个源模型到各 Provider 的目标模型映射
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelRoutingRow {
    pub source_model: String,
    /// provider_id → target_model（None 表示该 Provider 不参与此 source_model 的匹配）
    pub targets: HashMap<Uuid, Option<String>>,
}

impl ModelRoutingGrid {
    /// 对于给定的 provider_id 和请求模型，查找目标模型。
    /// 匹配优先级：精确匹配 > 前缀匹配（最长前缀优先）> __unmatched__ 兜底 > 返回原始模型
    pub fn resolve_model(&self, requested_model: &str, provider_id: &Uuid) -> String {
        // 1. 精确匹配
        for row in &self.rows {
            if row.source_model == requested_model {
                if let Some(target) = row.targets.get(provider_id).and_then(|t| t.clone()) {
                    return target;
                }
            }
        }

        // 2. 前缀匹配 — 收集所有候选，最长前缀优先
        //    前缀行通过 is_prefix_source_model() 识别：包括 Anthropic 模型族（claude-haiku- 等）
        //    和 __unmatched__ 哨兵值。以 - 结尾的族名自然形成前缀。
        let mut candidates: Vec<(&str, &Option<String>)> = Vec::new();
        for row in &self.rows {
            if row.source_model == UNMATCHED_MODEL {
                continue; // __unmatched__ 在步骤 3 单独处理
            }
            if is_prefix_source_model(&row.source_model)
                && requested_model.starts_with(&row.source_model)
            {
                if let Some(target) = row.targets.get(provider_id) {
                    candidates.push((&row.source_model, target));
                }
            }
        }
        // 按前缀长度降序排列，最长（最具体）匹配优先
        candidates.sort_by_key(|b| std::cmp::Reverse(b.0.len()));
        if let Some((_, target)) = candidates.first() {
            if let Some(t) = (*target).clone() {
                return t;
            }
        }

        // 3. __unmatched__ 兜底
        for row in &self.rows {
            if row.source_model == UNMATCHED_MODEL {
                if let Some(target) = row.targets.get(provider_id).and_then(|t| t.clone()) {
                    return target;
                }
            }
        }
        // 4. 返回原始模型
        requested_model.to_string()
    }

    /// 移除指定 provider_id 的列
    pub fn remove_provider_column(&mut self, provider_id: &Uuid) {
        self.provider_ids.retain(|id| id != provider_id);
        for row in &mut self.rows {
            row.targets.remove(provider_id);
        }
    }

    /// 确保所有 provider_ids 在每行的 targets 中都有条目
    pub fn sync_providers(&mut self) {
        for pid in &self.provider_ids.clone() {
            for row in &mut self.rows {
                row.targets.entry(*pid).or_insert(None);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_grid() -> ModelRoutingGrid {
        let pid1 = Uuid::new_v4();
        let pid2 = Uuid::new_v4();
        let mut row1 = ModelRoutingRow {
            source_model: "gpt-4".to_string(),
            targets: HashMap::new(),
        };
        row1.targets.insert(pid1, Some("gpt-4-turbo".to_string()));
        row1.targets.insert(pid2, Some("gpt-4-32k".to_string()));

        let mut row2 = ModelRoutingRow {
            source_model: "claude-sonnet-".to_string(),
            targets: HashMap::new(),
        };
        row2.targets
            .insert(pid1, Some("claude-sonnet-4".to_string()));

        let mut row3 = ModelRoutingRow {
            source_model: UNMATCHED_MODEL.to_string(),
            targets: HashMap::new(),
        };
        row3.targets
            .insert(pid1, Some("fallback-model".to_string()));

        ModelRoutingGrid {
            provider_ids: vec![pid1, pid2],
            rows: vec![row1, row2, row3],
        }
    }

    #[test]
    fn test_resolve_model_exact_match() {
        let grid = make_grid();
        let pid = grid.provider_ids[0];
        let result = grid.resolve_model("gpt-4", &pid);
        assert_eq!(result, "gpt-4-turbo");
    }

    #[test]
    fn test_resolve_model_exact_match_different_provider() {
        let grid = make_grid();
        let pid = grid.provider_ids[1];
        let result = grid.resolve_model("gpt-4", &pid);
        assert_eq!(result, "gpt-4-32k");
    }

    #[test]
    fn test_resolve_model_prefix_match() {
        let grid = make_grid();
        let pid = grid.provider_ids[0];
        let result = grid.resolve_model("claude-sonnet-4-20250514", &pid);
        assert_eq!(result, "claude-sonnet-4");
    }

    #[test]
    fn test_resolve_model_unmatched() {
        let grid = make_grid();
        let pid = grid.provider_ids[0];
        let result = grid.resolve_model("unknown-model", &pid);
        assert_eq!(result, "fallback-model");
    }

    #[test]
    fn test_resolve_model_anthropic_family_prefix() {
        let pid = Uuid::new_v4();
        // Anthropic 模型族前缀（如 claude-haiku-）通过 is_prefix_source_model() 识别
        let mut haiku_row = ModelRoutingRow {
            source_model: "claude-haiku-".to_string(),
            targets: HashMap::new(),
        };
        haiku_row
            .targets
            .insert(pid, Some("deepseek-v4-flash".to_string()));

        let mut sonnet_row = ModelRoutingRow {
            source_model: "claude-sonnet-".to_string(),
            targets: HashMap::new(),
        };
        sonnet_row
            .targets
            .insert(pid, Some("deepseek-v4-pro".to_string()));

        let grid = ModelRoutingGrid {
            provider_ids: vec![pid],
            rows: vec![haiku_row, sonnet_row],
        };
        // claude-haiku-4-5-20251001 应匹配 claude-haiku- 而非 claude-sonnet-
        assert_eq!(
            grid.resolve_model("claude-haiku-4-5-20251001", &pid),
            "deepseek-v4-flash"
        );
        // claude-sonnet-4-20250514 应匹配 claude-sonnet-
        assert_eq!(
            grid.resolve_model("claude-sonnet-4-20250514", &pid),
            "deepseek-v4-pro"
        );
    }

    #[test]
    fn test_resolve_model_no_match() {
        let grid = make_grid();
        let pid = grid.provider_ids[1]; // pid2 has no fallback
        let result = grid.resolve_model("unknown-model", &pid);
        assert_eq!(result, "unknown-model");
    }

    #[test]
    fn test_remove_provider_column() {
        let mut grid = make_grid();
        let pid = grid.provider_ids[0];
        grid.remove_provider_column(&pid);
        assert!(!grid.provider_ids.contains(&pid));
        for row in &grid.rows {
            assert!(!row.targets.contains_key(&pid));
        }
    }

    #[test]
    fn test_sync_providers() {
        let mut grid = ModelRoutingGrid {
            provider_ids: vec![Uuid::new_v4()],
            rows: vec![ModelRoutingRow {
                source_model: "test".to_string(),
                targets: HashMap::new(),
            }],
        };
        grid.sync_providers();
        assert!(grid.rows[0].targets.contains_key(&grid.provider_ids[0]));
        assert_eq!(grid.rows[0].targets[&grid.provider_ids[0]], None);
    }
}
