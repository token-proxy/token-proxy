import { useState, useEffect, useCallback, type ReactNode } from 'react';
import {
  Card, Form, Button, Toast, Typography, Spin,
} from '@douyinfe/semi-ui';
import api from '../api.ts';

const { Title } = Typography;

interface Settings {
  log_retention_months: number;
}

export default function SettingsPage(): ReactNode {
  const [settings, setSettings] = useState<Settings | null>(null);
  const [loading, setLoading] = useState(false);
  const [saving, setSaving] = useState(false);

  const loadSettings = useCallback(async () => {
    setLoading(true);
    try {
      const data = await api.get<Settings>('/api/settings');
      setSettings(data);
    } catch (err) {
      Toast.error(err instanceof Error ? err.message : '获取设置失败');
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadSettings();
  }, [loadSettings]);

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
      <Title heading={3} style={{ marginBottom: 24 }}>系统设置</Title>

      <Card>
        {loading && !settings ? (
          <div style={{ display: 'flex', justifyContent: 'center', padding: 40 }}>
            <Spin />
          </div>
        ) : (
          <Form
            onSubmit={handleSave}
            initValues={settings || undefined}
            style={{ maxWidth: 480 }}
          >
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
