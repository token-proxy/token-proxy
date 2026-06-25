import { type ReactNode, useCallback, useMemo, useState } from 'react';
import {
  Button,
  Card,
  InputNumber,
  Popconfirm,
  Spin,
  Table,
  Toast,
  Tooltip,
  Typography,
} from '@douyinfe/semi-ui';
import { settingsApi } from '../api';
import { useFetch } from '../hooks/useFetch';
import type { MonthlySummary, UpdateSettingsRequest } from '../types/settings';
import type { ColumnProps } from '@douyinfe/semi-ui/lib/es/table';

const { Title, Text } = Typography;

// ─── 工具函数 ───

/**
 * 按 1024 进制将字节数格式化为人类可读字符串。
 *
 * >= 1024³ → GiB，>= 1024² → MiB，>= 1024 → KiB，否则显示 B。
 * 例如 5368709120 → "5.00 GiB"，1048576 → "1.00 MiB"。
 */
function formatSize(bytes: number): string {
  if (bytes >= 1024 ** 3) return `${(bytes / 1024 ** 3).toFixed(2)} GiB`;
  if (bytes >= 1024 ** 2) return `${(bytes / 1024 ** 2).toFixed(2)} MiB`;
  if (bytes >= 1024) return `${(bytes / 1024).toFixed(2)} KiB`;
  return `${bytes} B`;
}

/** 当前月份标识（如 `2026-06`），用于屏蔽当前月删除 */
function getCurrentMonth(): string {
  return new Date().toISOString().slice(0, 7);
}

// ─── 表单行组件 ───

/** 表单行的通用布局：标签 + 内容（左对齐） */
function FormRow({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div style={{ display: 'flex', alignItems: 'center', minHeight: 32 }}>
      <Text type="secondary" style={{ flexShrink: 0, width: 96, fontSize: 13 }}>
        {label}
      </Text>
      <div style={{ flex: 1 }}>{children}</div>
    </div>
  );
}

// ─── 页面组件 ───

/**
 * SettingsPage - 系统设置页面
 *
 * 采用左右两栏布局：
 * - 左侧：设置表单（日志保留时长 / 已保留时长 / 日志存储上限 / 已存储日志）
 * - 右侧：月度数据表格（含占比列）+ 规则说明
 */
export default function SettingsPage(): ReactNode {
  // ─── 并行加载数据 ───

  const {
    data: logStats,
    loading: statsLoading,
    error: statsError,
    refetch: refetchStats,
  } = useFetch(() => settingsApi.getLogStats(), []);

  const {
    data: settings,
    loading: settingsLoading,
    refetch: refetchSettings,
  } = useFetch(() => settingsApi.getSettings(), []);

  // ─── 内联编辑状态 ───

  const [editing, setEditing] = useState(false);
  const [saving, setSaving] = useState(false);
  const [deletingMonth, setDeletingMonth] = useState<string | null>(null);

  const [formValues, setFormValues] = useState<UpdateSettingsRequest>({
    log_retention_months: 12,
    log_storage_cap_gb: null,
  });

  // ─── 操作回调 ───

  /** 进入编辑模式，从当前设置填充表单值 */
  const handleEdit = useCallback(() => {
    setFormValues({
      log_retention_months: settings?.log_retention_months ?? 12,
      log_storage_cap_gb: settings?.log_storage_cap_gb ?? null,
    });
    setEditing(true);
  }, [settings]);

  /** 取消编辑，还原表单值为当前设置 */
  const handleCancel = useCallback(() => {
    setEditing(false);
    setFormValues({
      log_retention_months: settings?.log_retention_months ?? 12,
      log_storage_cap_gb: settings?.log_storage_cap_gb ?? null,
    });
  }, [settings]);

  /** 保存设置 */
  const handleSave = useCallback(async () => {
    // 校验
    if (formValues.log_retention_months < 1 || formValues.log_retention_months > 36) {
      Toast.error('日志保留时长必须在 1 到 36 个月之间');
      return;
    }
    if (
      formValues.log_storage_cap_gb !== null &&
      (formValues.log_storage_cap_gb < 1 || formValues.log_storage_cap_gb > 10000)
    ) {
      Toast.error('日志存储上限必须在 1 到 10000 GiB 之间');
      return;
    }

    setSaving(true);
    try {
      await settingsApi.updateSettings(formValues);
      Toast.success('日志设置已保存');
      setEditing(false);
      refetchSettings();
    } catch (err) {
      Toast.error(err instanceof Error ? err.message : '保存设置失败');
    } finally {
      setSaving(false);
    }
  }, [formValues, refetchSettings]);

  /** 删除指定月份的日志分区 */
  const handleDeleteMonth = useCallback(
    async (month: string) => {
      setDeletingMonth(month);
      try {
        const result = await settingsApi.deleteMonthLogs(month);
        Toast.success(result.message);
        refetchStats();
      } catch (err) {
        Toast.error(err instanceof Error ? err.message : '删除失败');
      } finally {
        setDeletingMonth(null);
      }
    },
    [refetchStats],
  );

  // ─── 派生数据 ───

  const currentMonth = useMemo(() => getCurrentMonth(), []);

  /** 月度表格数据，前端计算百分比 */
  const monthlyDataWithPct = useMemo(() => {
    if (!logStats?.monthly_summary) return [];
    const total = logStats.total_size_bytes || 1;
    return logStats.monthly_summary.map((m) => ({
      ...m,
      _percent: ((m.size_bytes / total) * 100).toFixed(1),
    }));
  }, [logStats]);

  // ─── 月度表格列定义 ───

  const monthlyColumns = useMemo<ColumnProps<MonthlySummary & { _percent: string }>[]>(
    () => [
      { title: '月份', dataIndex: 'month', width: 90 },
      {
        title: '磁盘占用',
        dataIndex: 'size_bytes',
        width: 110,
        render: (_: unknown, record: MonthlySummary) => formatSize(record.size_bytes),
      },
      {
        title: '占比',
        width: 70,
        render: (_: unknown, record: MonthlySummary & { _percent: string }) =>
          `${record._percent}%`,
      },
      {
        title: '行数估算',
        dataIndex: 'row_count_estimate',
        width: 100,
        render: (_: unknown, record: MonthlySummary) => record.row_count_estimate.toLocaleString(),
      },
      {
        title: '操作',
        dataIndex: 'month',
        width: 90,
        render: (_: unknown, record: MonthlySummary) => {
          const isCurrentMonth = record.month === currentMonth;
          if (isCurrentMonth) {
            return (
              <Tooltip content="当前月份的日志分区不可删除">
                <Button type="danger" size="small" disabled>
                  删除
                </Button>
              </Tooltip>
            );
          }

          return (
            <Popconfirm
              title="确认删除"
              content={`确认删除 ${record.month} 月的日志分区？此操作不可撤销。`}
              onConfirm={() => handleDeleteMonth(record.month)}
              okButtonProps={{
                type: 'danger',
                loading: deletingMonth === record.month,
              }}
            >
              <Button type="danger" size="small" loading={deletingMonth === record.month}>
                删除
              </Button>
            </Popconfirm>
          );
        },
      },
    ],
    [currentMonth, deletingMonth, handleDeleteMonth],
  );

  // ─── 加载状态 ───

  const isLoading = (statsLoading && !logStats) || (settingsLoading && !settings);

  if (isLoading) {
    return (
      <div>
        <Title heading={3} style={{ marginBottom: 24 }}>
          系统设置
        </Title>
        <Card title="日志管理">
          <div style={{ display: 'flex', justifyContent: 'center', padding: 40 }}>
            <Spin />
          </div>
        </Card>
      </div>
    );
  }

  return (
    <div>
      <Title heading={3} style={{ marginBottom: 24 }}>
        系统设置
      </Title>

      <Card title="日志管理">
        {/* 左右两栏布局 */}
        <div style={{ display: 'flex', gap: 32 }}>
          {/* ─── 左侧面板（设置表单） ─── */}
          <div style={{ flex: 1 }}>
            <div style={{ display: 'flex', flexDirection: 'column', gap: 20 }}>
              {/* 日志保留时长（可编辑） */}
              <FormRow label="日志保留时长">
                <InputNumber
                  value={formValues.log_retention_months}
                  onChange={(v) =>
                    setFormValues((prev) => ({
                      ...prev,
                      log_retention_months: (v as number) ?? 12,
                    }))
                  }
                  min={1}
                  max={36}
                  disabled={!editing}
                  hideButtons
                  suffix="月"
                  style={{ width: 160 }}
                />
              </FormRow>

              {/* 已保留时长（只读） */}
              <FormRow label="已保留时长">
                <Text style={{ fontSize: 14 }}>{settings?.log_month_count ?? 0} 个月</Text>
              </FormRow>

              {/* 日志存储上限（可编辑） */}
              <FormRow label="日志存储上限">
                <InputNumber
                  value={formValues.log_storage_cap_gb ?? undefined}
                  onChange={(v) =>
                    setFormValues((prev) => ({
                      ...prev,
                      log_storage_cap_gb: v === undefined || v === null ? null : (v as number),
                    }))
                  }
                  min={1}
                  max={10000}
                  disabled={!editing}
                  hideButtons
                  suffix="GiB"
                  placeholder="不限制"
                  style={{ width: 160 }}
                />
              </FormRow>

              {/* 已存储日志（只读） */}
              <FormRow label="已存储日志">
                <Text style={{ fontSize: 14 }}>{formatSize(settings?.total_size_bytes ?? 0)}</Text>
              </FormRow>
            </div>

            {/* 操作区 */}
            <div style={{ marginTop: 24 }}>
              {!editing ? (
                <Button type="primary" onClick={handleEdit}>
                  更改
                </Button>
              ) : (
                <div style={{ display: 'flex', gap: 8 }}>
                  <Button type="primary" loading={saving} onClick={handleSave}>
                    保存
                  </Button>
                  <Button type="tertiary" onClick={handleCancel}>
                    取消
                  </Button>
                </div>
              )}
            </div>
          </div>

          {/* ─── 右侧面板（月度表格 + 规则说明） ─── */}
          <div style={{ flex: 1.5 }}>
            {statsError ? (
              <div style={{ padding: 16, color: 'var(--semi-color-danger)' }}>
                <Text type="danger">
                  获取日志统计失败: {statsError}，请
                  <Button
                    type="tertiary"
                    size="small"
                    onClick={refetchStats}
                    style={{ padding: 0, marginLeft: 4 }}
                  >
                    重试
                  </Button>
                </Text>
              </div>
            ) : (
              <>
                <Table<MonthlySummary & { _percent: string }>
                  columns={monthlyColumns}
                  dataSource={monthlyDataWithPct}
                  pagination={false}
                  size="small"
                  rowKey="month"
                  empty={<Text type="tertiary">暂无日志数据</Text>}
                />

                {/* 占用上限规则说明 */}
                <Text
                  type="tertiary"
                  style={{
                    fontSize: 12,
                    marginTop: 12,
                    display: 'block',
                  }}
                >
                  设置日志存储上限后，系统将在每次分区维护周期（默认每小时）中自动检查已存储日志总量。
                  若超出上限，将从最早月份开始清理日志分区，直到占用低于上限。
                  当前月份的分区不会被清理。
                </Text>
              </>
            )}
          </div>
        </div>
      </Card>
    </div>
  );
}
