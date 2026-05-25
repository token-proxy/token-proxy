import { useState, useEffect, useCallback, type ReactNode } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import {
  Breadcrumb,
  Button,
  Card,
  Collapse,
  Descriptions,
  Empty,
  Skeleton,
  TabPane,
  Tabs,
  Tag,
  Toast,
  Typography,
} from '@douyinfe/semi-ui';
import { IconArrowLeft, IconCopy } from '@douyinfe/semi-icons';
import type { LogDetailFull } from '../types/log.ts';
import api from '../api.ts';
import { formatDateTime, formatNumber } from '../utils/format.ts';
import CopyableIdText from '../components/CopyableIdText.tsx';

const { Text, Paragraph } = Typography;

// ─── Constants ───

/** 敏感请求头 key（大小写不敏感） */
const SENSITIVE_HEADER_KEYS = new Set([
  'authorization',
  'x-api-key',
  'api-key',
  'proxy-authorization',
]);

const SOURCE_LABELS: Record<string, string> = {
  main: '主代理',
  subagent: '子代理',
  unknown: '未知',
};

const SOURCE_COLORS = {
  main: 'blue',
  subagent: 'green',
  unknown: 'grey',
} as const;

// ─── Helpers ───

/**
 * 格式化请求头 JSON，将敏感 header 的值替换为 [已隐藏]
 */
function formatHeadersWithMasking(
  headers: Record<string, unknown> | null | undefined,
): string {
  if (!headers) return '(无请求头)';

  return Object.entries(headers)
    .map(([key, value]) => {
      if (SENSITIVE_HEADER_KEYS.has(key.toLowerCase())) {
        return `${key}: [已隐藏]`;
      }
      return `${key}: ${String(value)}`;
    })
    .join('\n');
}

/**
 * 格式化耗时：< 1000ms 显示 "xxx ms"，>= 1000ms 显示 "x.x s"
 */
function formatDurationDetail(ms: number | null | undefined): string {
  if (ms === null || ms === undefined) return '-';
  if (ms >= 1000) {
    return `${(ms / 1000).toFixed(1)} s`;
  }
  return `${ms} ms`;
}

// ─── Component ───

export default function LogDetailPage(): ReactNode {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();

  const [data, setData] = useState<LogDetailFull | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [copyingHeaders, setCopyingHeaders] = useState(false);
  const [copyingRawResponse, setCopyingRawResponse] = useState(false);

  // ── 加载数据 ──

  const fetchDetail = useCallback(async () => {
    if (!id) {
      setError('日志 ID 不存在');
      setLoading(false);
      return;
    }

    setLoading(true);
    setError(null);
    try {
      const result = await api.get<LogDetailFull>(`/api/logs/${id}/detail`);
      setData(result);
    } catch (err) {
      setError(err instanceof Error ? err.message : '加载日志详情失败');
    } finally {
      setLoading(false);
    }
  }, [id]);

  useEffect(() => {
    fetchDetail();
  }, [fetchDetail]);

  // ── 复制处理 ──

  const handleCopy = async (
    text: string,
    setCopying: (v: boolean) => void,
  ) => {
    setCopying(true);
    try {
      await navigator.clipboard.writeText(text);
      Toast.success('已复制到剪贴板');
    } catch {
      Toast.error('复制失败，请手动复制');
    } finally {
      setCopying(false);
    }
  };

  // ── 构建 Descriptions 数据 ──

  const buildBasicInfoData = (d: LogDetailFull) => {
    const items: Array<{ key: string; value: ReactNode }> = [
      { key: '请求 ID', value: <CopyableIdText value={d.id} /> },
      { key: '时间', value: formatDateTime(d.timestamp) },
      { key: '会话 ID', value: <CopyableIdText value={d.session_id} /> },
    ];

    if (d.user_id) {
      items.push({ key: '用户', value: d.user_id });
    }
    if (d.access_point_id) {
      items.push({
        key: '接入点 ID',
        value: <CopyableIdText value={d.access_point_id} />,
      });
    }

    items.push({
      key: '模型映射',
      value: (
        <span>
          <span className="monospace-text">{d.model_original || '-'}</span>
          {' → '}
          <span className="monospace-text">{d.model_mapped || '-'}</span>
        </span>
      ),
    });

    items.push({
      key: '状态码',
      value: (
        <Tag
          color={(d.status_code ?? 0) >= 400 ? 'red' : 'green'}
          size="small"
        >
          {d.status_code ?? '-'}
        </Tag>
      ),
    });

    items.push({
      key: '耗时',
      value: formatDurationDetail(d.duration_ms),
    });

    items.push({
      key: '请求序号',
      value: String(d.request_index),
    });

    items.push({
      key: '来源',
      value: (
        <Tag
          color={(SOURCE_COLORS[d.conversation_source as keyof typeof SOURCE_COLORS] || 'grey')}
          size="small"
        >
          {SOURCE_LABELS[d.conversation_source] || d.conversation_source}
        </Tag>
      ),
    });

    if (d.agent_id) {
      items.push({
        key: 'Agent ID',
        value: <CopyableIdText value={d.agent_id} />,
      });
    }
    if (d.agent_type) {
      items.push({ key: 'Agent 类型', value: d.agent_type });
    }

    // 客户端信息
    const clientParts = [
      d.client_name,
      d.client_version,
      d.client_channel,
      d.client_platform,
    ].filter(Boolean);
    if (clientParts.length > 0) {
      items.push({ key: '客户端', value: clientParts.join(' / ') });
    }

    return items;
  };

  const hasTokenData = (
    data: LogDetailFull,
  ): boolean =>
    data.token_input_tokens != null ||
    data.token_output_tokens != null ||
    data.token_cache_creation_input_tokens != null ||
    data.token_cache_read_input_tokens != null ||
    data.token_thinking_tokens != null ||
    data.token_total_tokens != null;

  const buildTokenData = (d: LogDetailFull) => [
    {
      key: '输入 Tokens',
      value: formatNumber(d.token_input_tokens ?? 0),
    },
    {
      key: '输出 Tokens',
      value: formatNumber(d.token_output_tokens ?? 0),
    },
    {
      key: '缓存创建',
      value: formatNumber(d.token_cache_creation_input_tokens ?? 0),
    },
    {
      key: '缓存读取',
      value: formatNumber(d.token_cache_read_input_tokens ?? 0),
    },
    {
      key: '思考 Tokens',
      value: formatNumber(d.token_thinking_tokens ?? 0),
    },
    { key: '总计', value: formatNumber(d.token_total_tokens ?? 0) },
  ];

  // ── 加载中状态 ──

  if (loading) {
    return (
      <div style={{ padding: 24 }}>
        <Skeleton
          active
          placeholder={<Skeleton.Title style={{ width: 200 }} />}
          style={{ marginBottom: 24 }}
        />
        <Card>
          <Skeleton
            active
            placeholder={<Skeleton.Paragraph rows={6} />}
          />
        </Card>
        <div style={{ marginTop: 16 }}>
          <Card>
            <Skeleton
              active
              placeholder={<Skeleton.Paragraph rows={4} />}
            />
          </Card>
        </div>
      </div>
    );
  }

  // ── 错误状态 ──

  if (error) {
    return (
      <div style={{ padding: 24 }}>
        <Empty
          description={error}
          style={{ marginTop: 80 }}
        >
          <Button type="primary" onClick={fetchDetail}>
            重试
          </Button>
        </Empty>
      </div>
    );
  }

  // ── 数据为空状态 ──

  if (!data) {
    return (
      <div style={{ padding: 24 }}>
        <Empty
          description="未找到日志详情"
          style={{ marginTop: 80 }}
        />
      </div>
    );
  }

  // ── 数据展示 ──

  const headersText = formatHeadersWithMasking(
    data.request_headers as Record<string, unknown>,
  );

  return (
    <div style={{ padding: 24 }}>
      {/* ── 面包屑导航 ── */}
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: 12,
          marginBottom: 16,
        }}
      >
        <Breadcrumb>
          <Breadcrumb.Item>
            <span
              style={{
                cursor: 'pointer',
                color: 'var(--semi-color-text-2)',
              }}
              onClick={() => navigate('/logs')}
            >
              日志列表
            </span>
          </Breadcrumb.Item>
          <Breadcrumb.Item>请求详情</Breadcrumb.Item>
        </Breadcrumb>
      </div>

      {/* ── 返回按钮 ── */}
      <div style={{ marginBottom: 20 }}>
        <Button
          icon={<IconArrowLeft />}
          type="tertiary"
          onClick={() => navigate('/logs')}
        >
          返回日志列表
        </Button>
      </div>

      {/* ── 1. 基础信息卡片 ── */}
      <Card
        title="基础信息"
        style={{ marginBottom: 16 }}
        bodyStyle={{ padding: '20px 24px' }}
      >
        <Descriptions
          data={buildBasicInfoData(data)}
          row
          size="small"
        />
      </Card>

      {/* ── 2. Token 用量卡片 ── */}
      {hasTokenData(data) && (
        <Card
          title="Token 用量"
          style={{ marginBottom: 16 }}
          bodyStyle={{ padding: '20px 24px' }}
        >
          <Descriptions
            data={buildTokenData(data)}
            row
            size="small"
          />
        </Card>
      )}

      {/* ── 3. 请求头区域（默认折叠） ── */}
      <Collapse style={{ marginBottom: 16 }} defaultActiveKey={[]}>
        <Collapse.Panel
          header="请求头"
          itemKey="headers"
        >
          <div
            style={{
              display: 'flex',
              justifyContent: 'flex-end',
              marginBottom: 8,
            }}
          >
            <Button
              icon={<IconCopy />}
              size="small"
              type="tertiary"
              loading={copyingHeaders}
              disabled={copyingRawResponse}
              onClick={() => handleCopy(headersText, setCopyingHeaders)}
            >
              复制
            </Button>
          </div>
          <pre
            style={{
              background: 'var(--semi-color-fill-0)',
              padding: 12,
              borderRadius: 4,
              fontSize: 12,
              overflow: 'auto',
              maxHeight: 300,
              whiteSpace: 'pre-wrap',
              wordBreak: 'break-all',
              margin: 0,
            }}
          >
            {headersText}
          </pre>
        </Collapse.Panel>
      </Collapse>

      {/* ── 4. 请求内容卡片 ── */}
      <Card
        title="请求内容"
        style={{ marginBottom: 16 }}
        bodyStyle={{ padding: '20px 24px' }}
      >
        {data.request_message_text ? (
          <Paragraph
            ellipsis={{ rows: 5, expandable: true, collapsible: true }}
            style={{ margin: 0, whiteSpace: 'pre-wrap' }}
          >
            {data.request_message_text}
          </Paragraph>
        ) : (
          <Text type="secondary">(无请求消息文本)</Text>
        )}

        {data.request_body &&
          Object.keys(data.request_body).length > 0 && (
            <div style={{ marginTop: 12 }}>
              <Text
                strong
                style={{ display: 'block', marginBottom: 4 }}
              >
                请求体:
              </Text>
              <pre
                style={{
                  background: 'var(--semi-color-fill-0)',
                  padding: 12,
                  borderRadius: 4,
                  fontSize: 12,
                  overflow: 'auto',
                  maxHeight: 300,
                  whiteSpace: 'pre-wrap',
                  wordBreak: 'break-all',
                  margin: 0,
                }}
              >
                {JSON.stringify(data.request_body, null, 2)}
              </pre>
            </div>
          )}
      </Card>

      {/* ── 5. 响应内容卡片 ── */}
      <Card
        title="响应内容"
        bodyStyle={{ padding: '20px 24px' }}
      >
        <Tabs type="card">
          {/* 结构化视图标签页 */}
          <TabPane tab="结构化视图" itemKey="structured">
            {data.response_assistant_text ? (
              <div style={{ marginBottom: 16 }}>
                <Text
                  strong
                  style={{ display: 'block', marginBottom: 8 }}
                >
                  助手回复:
                </Text>
                <Paragraph
                  ellipsis={{
                    rows: 10,
                    expandable: true,
                    collapsible: true,
                  }}
                  style={{ margin: 0, whiteSpace: 'pre-wrap' }}
                >
                  {data.response_assistant_text}
                </Paragraph>
              </div>
            ) : (
              <Text
                type="secondary"
                style={{ display: 'block', marginBottom: 16 }}
              >
                (无助手回复)
              </Text>
            )}

            {data.response_thinking_text ? (
              <Collapse defaultActiveKey={[]}>
                <Collapse.Panel
                  header="思考内容"
                  itemKey="thinking"
                >
                  <pre
                    style={{
                      background: 'var(--semi-color-fill-0)',
                      padding: 12,
                      borderRadius: 4,
                      fontSize: 12,
                      overflow: 'auto',
                      maxHeight: 400,
                      whiteSpace: 'pre-wrap',
                      wordBreak: 'break-all',
                      margin: 0,
                    }}
                  >
                    {data.response_thinking_text}
                  </pre>
                </Collapse.Panel>
              </Collapse>
            ) : (
              <Text type="secondary">(无思考内容)</Text>
            )}
          </TabPane>

          {/* 原始 SSE 标签页 */}
          <TabPane tab="原始 SSE" itemKey="raw">
            <div
              style={{
                display: 'flex',
                justifyContent: 'flex-end',
                marginBottom: 8,
              }}
            >
              <Button
                icon={<IconCopy />}
                size="small"
                type="tertiary"
                loading={copyingRawResponse}
                disabled={copyingHeaders}
                onClick={() =>
                  handleCopy(data.response_body, setCopyingRawResponse)
                }
              >
                复制
              </Button>
            </div>
            <pre
              style={{
                background: 'var(--semi-color-fill-0)',
                padding: 12,
                borderRadius: 4,
                fontSize: 12,
                overflow: 'auto',
                maxHeight: 500,
                whiteSpace: 'pre-wrap',
                wordBreak: 'break-all',
                margin: 0,
              }}
            >
              {data.response_body || '(空)'}
            </pre>
          </TabPane>
        </Tabs>
      </Card>
    </div>
  );
}