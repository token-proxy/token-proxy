/**
 * 审计日志工具函数模块。
 *
 * 提供审计操作类型与实体类型的标签映射、Select 选项等辅助工具。
 */

// ─── 标签映射 ───

/** AuditAction → 中文显示映射 */
export const ACTION_LABELS: Record<string, string> = {
  create: '创建',
  update: '更新',
  delete: '删除',
  enable: '启用',
  disable: '禁用',
  recover: '恢复',
  auto_recover: '自动恢复',
  create_api_key: '创建 API Key',
  revoke_api_key: '吊销 API Key',
  update_api_key_description: '更新 API Key 描述',
  change_password: '修改密码',
  update_profile: '更新个人信息',
  update_settings: '更新系统设置',
  login: '登录',
  login_failed: '登录失败',
  logout: '登出',
  refresh_rejected: '刷新令牌被拒绝',
  discover_models: '发现模型',
};

/** AuditEntityType → 中文显示映射 */
export const ENTITY_TYPE_LABELS: Record<string, string> = {
  access_point: '接入点',
  account: '账号',
  provider: '服务商',
  user: '用户',
  user_api_key: '用户 API Key',
  system_settings: '系统设置',
  auth_session: '认证会话',
  refresh_token: '刷新令牌',
};

// ─── Select 选项（供多选下拉使用） ───

/** 操作类型选项（用于 Select 多选组件） */
export const ACTION_OPTIONS = Object.entries(ACTION_LABELS).map(([value, label]) => ({
  value,
  label,
}));

/** 实体类型选项（用于 Select 多选组件） */
export const ENTITY_TYPE_OPTIONS = Object.entries(ENTITY_TYPE_LABELS).map(([value, label]) => ({
  value,
  label,
}));
