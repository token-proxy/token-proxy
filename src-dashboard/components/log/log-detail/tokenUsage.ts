import type { LogDetailFull } from '../../../types/log.ts';

/** 判断日志是否包含 Token 用量数据 */
export function hasTokenData(d: LogDetailFull): boolean {
  return (
    d.token_input_tokens != null ||
    d.token_output_tokens != null ||
    d.token_cache_creation_input_tokens != null ||
    d.token_cache_read_input_tokens != null ||
    d.token_thinking_tokens != null ||
    d.token_total_tokens != null
  );
}
