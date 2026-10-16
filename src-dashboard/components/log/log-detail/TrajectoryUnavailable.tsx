import { type ReactNode } from 'react';
import { Card, Tag, Typography } from '@douyinfe/semi-ui';
import { IconInfoCircle } from '@douyinfe/semi-icons';
import CodeHighlight from '@components/common/CodeHighlight';
import type { LogDetailFull } from '../../../types/log.ts';
import './TrajectoryUnavailable.css';

const { Title, Paragraph, Text } = Typography;

/** 原文不可用的成因 */
export type TrajectoryUnavailableReason = 'cleaned' | 'empty';

/** TrajectoryUnavailable 组件 Props */
interface TrajectoryUnavailableProps {
  /** 日志完整详情（仅用到仍可用的元信息与上游原始 usage） */
  data: LogDetailFull;
  /** 成因：cleaned = 原文记录已被清理；empty = 原文记录存在但内容为空 */
  reason: TrajectoryUnavailableReason;
  style?: React.CSSProperties;
}

/**
 * TrajectoryUnavailable - 运行轨迹不可用说明
 *
 * 请求体与响应体都没有内容时替代轨迹视图：继续画空轨迹只会误导阅读
 * （时间轴只有两根条、账本只有两个信封行），这里明确说明缺失原因、
 * 仍然可用的信息与无法还原的内容，并展示仍保留的上游原始 usage。
 */
export default function TrajectoryUnavailable({
  data,
  reason,
  style,
}: TrajectoryUnavailableProps): ReactNode {
  const cleaned = reason === 'cleaned';

  return (
    <Card
      style={style}
      bodyStyle={{ padding: '20px 24px' }}
      title={
        <span className="trajectory-unavailable__title">
          运行轨迹不可用
          <Tag size="small" color="grey">
            {cleaned ? '原文已清理' : '原文为空'}
          </Tag>
        </span>
      }
    >
      <div className="trajectory-unavailable">
        <div className="trajectory-unavailable__lead">
          <IconInfoCircle size="large" />
          <span>
            {cleaned
              ? '这条日志的请求 / 响应原文没有保存在数据库中，没有可还原的对话轨迹、请求内容与响应内容。'
              : '这条日志的原文记录存在但内容为空（常见于客户端中途断开或内容写入失败），没有可还原的对话轨迹与请求 / 响应内容。'}
          </span>
        </div>

        <section className="trajectory-unavailable__section">
          <Title heading={6}>为什么会缺失</Title>
          <Paragraph className="trajectory-unavailable__text">
            {cleaned ? (
              <>
                原始内容（请求头、请求体、响应头、响应体）存放在按月分区的{' '}
                <Text code>log_contents</Text>{' '}
                表，并按保留期清理；请求标量与词元用量存放在长期保留的{' '}
                <Text code>log_requests</Text>{' '}
                表。分区被清理、数据库重建或历史数据导入后未回补分区时，原文即不可恢复 ——
                本条记录正属于这种情况，不是解析失败。
              </>
            ) : (
              <>
                请求行本身没有携带可用的请求体或响应体：流式请求在客户端断开时可能只留下部分内容，
                内容写入失败时也会留下空记录。此时请求标量与词元用量仍然完整。
              </>
            )}
          </Paragraph>
        </section>

        <section className="trajectory-unavailable__section">
          <Title heading={6}>仍然可用的信息</Title>
          <Paragraph className="trajectory-unavailable__text">
            上方「基础信息」与「词元用量」完全可用：时间、模型映射、状态码、耗时、6 类词元，
            以及下方仍保留在 <Text code>log_requests.raw_usage</Text> 里的上游原始 usage。
            该记录也照常参与用量统计与看板聚合。
          </Paragraph>
          {data.token_raw_usage && (
            <div className="trajectory-unavailable__raw">
              <CodeHighlight content={JSON.stringify(data.token_raw_usage, null, 2)} />
            </div>
          )}
        </section>

        <section className="trajectory-unavailable__section">
          <Title heading={6}>无法还原的内容</Title>
          <ul className="trajectory-unavailable__list">
            <li>对话轨迹：系统提示、历史消息、工具调用与工具结果、助手回复</li>
            <li>请求头 / 响应头，以及请求体、响应体原文</li>
          </ul>
        </section>
      </div>
    </Card>
  );
}
