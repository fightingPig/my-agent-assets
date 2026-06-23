export type ExpectedVisualQaReport = {
  pageId: string;
  platform: string;
  width: number;
  height: number;
};

export function isExpectedVisualQaReport(
  report: unknown,
  expected: ExpectedVisualQaReport,
): boolean;
