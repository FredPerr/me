import { LOCAL_REPORT_LIMIT } from './config.ts';
import { truncateMiddle } from './format-path.ts';
import type { ModuleMetric } from './types.ts';

export function printLocalReport(metrics: ModuleMetric[]): void {
  console.log(`\n🔥 TOP ${LOCAL_REPORT_LIMIT} CODEBASE BOTTLENECK MODULES 🔥\n`);
  console.table(metrics.slice(0, LOCAL_REPORT_LIMIT).map(toReportRow));
}

function toReportRow(metric: ModuleMetric): Record<string, string | number> {
  return {
    'File Path': truncateMiddle(metric.file),
    'Direct Imports': metric.directCount,
    'Blast Radius (Transitive)': metric.blastRadius,
    'Risk Score (0-100)': metric.score,
  };
}
