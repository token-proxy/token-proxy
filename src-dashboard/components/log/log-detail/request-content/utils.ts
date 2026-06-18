export function extractContentBlocks(
  content: unknown,
): Array<Record<string, unknown>> {
  if (Array.isArray(content)) return content as Array<Record<string, unknown>>;
  if (typeof content === 'string') {
    return [{type: 'text', text: content}];
  }
  return [];
}
