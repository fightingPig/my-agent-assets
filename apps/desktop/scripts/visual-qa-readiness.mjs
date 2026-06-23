export function isExpectedVisualQaReport(report, expected) {
  return report !== null
    && typeof report === "object"
    && report.pageId === expected.pageId
    && report.platform === expected.platform
    && report.viewport !== null
    && typeof report.viewport === "object"
    && report.viewport.width === expected.width
    && report.viewport.height === expected.height;
}
