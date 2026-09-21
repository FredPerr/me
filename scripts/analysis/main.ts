import { parseCliArgs } from './cli.ts';
import { MIN_FILES_TO_COMMENT, RISK_SCORE_THRESHOLD, ignoreListPath } from './config.ts';
import { loadDependencyGraph } from './dependency-graph.ts';
import { IgnoreList } from './ignore-list.ts';
import { computeMetrics } from './metrics.ts';
import { printLocalReport } from './local-report.ts';
import { buildAllClearComment, buildFindingsComment } from './comment.ts';
import { publishComment } from './report-writer.ts';
import type { CliArgs, ModuleMetric } from './types.ts';

function main(): void {
  const args = parseCliArgs(process.argv);
  const metrics = computeMetrics(loadDependencyGraph());

  if (args.isLocal) {
    printLocalReport(metrics);
    return;
  }
  runCiReport(metrics, args);
}

function run(): void {
  try {
    main();
  } catch (error) {
    console.error(`Error: ${error instanceof Error ? error.message : String(error)}`);
    process.exit(1);
  }
}

function runCiReport(metrics: ModuleMetric[], args: CliArgs): void {
  if (args.changedFiles.length === 0) return;

  const touchedBottlenecks = findTouchedBottlenecks(metrics, args.changedFiles);
  publishComment(selectCommentBody(touchedBottlenecks, args.changedFiles.length));
}

function findTouchedBottlenecks(metrics: ModuleMetric[], changedFiles: string[]): ModuleMetric[] {
  const ignoreList = IgnoreList.fromFile(ignoreListPath(import.meta.dirname));
  return metrics.filter(
    metric =>
      changedFiles.includes(metric.file) &&
      metric.score > RISK_SCORE_THRESHOLD &&
      !ignoreList.isStable(metric.file),
  );
}

function selectCommentBody(touchedBottlenecks: ModuleMetric[], analyzedCount: number): string {
  const hasFindings = touchedBottlenecks.length >= MIN_FILES_TO_COMMENT;
  return hasFindings
    ? buildFindingsComment(touchedBottlenecks, analyzedCount)
    : buildAllClearComment(analyzedCount);
}

run();
