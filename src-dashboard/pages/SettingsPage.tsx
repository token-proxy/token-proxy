import { useState, useEffect, useCallback, type ReactNode } from 'react';
import {
  Card, Form, Button, Toast, Typography, Spin,
} from '@douyinfe/semi-ui';
import api from '../api.ts';

const { Title } = Typography;

interface Settings {
  log_retention_days: number;
  stats_retention_days: number;
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
              field="log_retention_days"
              label="日志数据保留天数"
              extraText="日志元数据和内容（含请求/响应体）的保留期限"
              min={7}
              max={365}
            />
            <Form.InputNumber
              field="stats_retention_days"
              label="统计数据保留天数"
              extraText="物化视图聚合统计数据的保留期限"
              min={30}
              max={730}
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
