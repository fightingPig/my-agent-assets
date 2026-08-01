export type StatusTone = "success" | "warning" | "neutral" | "danger";

/** Maps user-facing status copy to the shared semantic presentation tones. */
export function statusToneForLabel(status: string): StatusTone {
  const normalized = status.toLocaleLowerCase();
  if (/(异常|失败|错误|冲突|需处理|阻止)/.test(status) || /(error|fail|conflict)/.test(normalized)) {
    return "danger";
  }
  if (/(未初始化|需注意|有变更|待同步|待处理|警告)/.test(status) || /(warning|pending|uninitialized)/.test(normalized)) {
    return "warning";
  }
  if (/(未连接|未读取|读取中|未发现|暂无|示例数据|只读|隔离)/.test(status) || /(unknown|not|loading|demo|isolated)/.test(normalized)) {
    return "neutral";
  }
  return "success";
}
