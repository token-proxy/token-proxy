import { type ReactNode, useState } from 'react';
import { useFetch } from '../hooks/useFetch.ts';
import { Button, Card, Form, Spin, Toast, Typography } from '@douyinfe/semi-ui';
import api from '../api.ts';

const { Title } = Typography;

interface Settings {
  log_retention_months: number;
}

/**
 * SettingsPage - 系统设置页面
 *
 * 提供全局系统配置的查看和修改功能（如日志保留月数）。
 */
export default function SettingsPage(): ReactNode {
  const {
    data: settings,
    loading,
    refetch: loadSettings,
  } = useFetch(() => api.get<Settings>('/api/settings'), []);
  const [saving, setSaving] = useState(false);

  const handleSave = async (values: Settings) => {
    setSaving(true);
    try {
      await api.put('/api/settings', values);
      Toast.success('设置已保存');
      loadSettings();
    } catch (err) {
      Toast.error(err instanceof Error ? err.message : '保存设置失败');
    } finally {
      setSaving(false);
    }
  };

  return (
    <div>
      <Title heading={3} style={{ marginBottom: 24 }}>
        系统设置
      </Title>

      <Card>
        {loading && !settings ? (
          <div style={{ display: 'flex', justifyContent: 'center', padding: 40 }}>
            <Spin />
          </div>
        ) : (
          <Form onSubmit={handleSave} initValues={settings || undefined} style={{ maxWidth: 480 }}>
            <Form.InputNumber
              field="log_retention_months"
              label="日志数据保留月数"
              extraText="按月分区存储，到期自动清理（包含日志元数据和请求/响应体）"
              min={1}
              max={36}
            />
            <Button type="primary" htmlType="submit" loading={saving} style={{ marginTop: 16 }}>
              保存
            </Button>
          </Form>
        )}
      </Card>
    </div>
  );
}
