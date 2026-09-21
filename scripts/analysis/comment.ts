import {
  COMMENT_MARKER,
  HIGH_RISK_SCORE,
  MIN_FILES_TO_COMMENT,
  RISK_SCORE_THRESHOLD,
} from './config.ts';
import type { ModuleMetric } from './types.ts';

export function buildFindingsComment(
  touchedBottlenecks: ModuleMetric[],
  analyzedCount: number,
): string {
  return (
    `${COMMENT_MARKER}\n` +
    `### ⚠️ Boy-Scouting Opportunity: High-Impact Files Modified\n\n` +
    `${analyzedFilesLine(analyzedCount)}\n\n` +
    `This PR modifies ${touchedBottlenecks.length} files with a bottleneck risk score above ${RISK_SCORE_THRESHOLD}. Changes here carry a wide blast radius across the codebase:\n\n` +
    findingsTable(touchedBottlenecks)
  );
}

export function buildAllClearComment(analyzedCount: number): string {
  return (
    `${COMMENT_MARKER}\n` +
    `### ✅ No Boy-Scouting Opportunity Detected\n\n` +
    `${analyzedFilesLine(analyzedCount)}\n\n` +
    `This PR does not modify ${MIN_FILES_TO_COMMENT} or more files with a bottleneck risk score above ${RISK_SCORE_THRESHOLD}. No high blast-radius hotspots were touched.\n`
  );
}

function findingsTable(touchedBottlenecks: ModuleMetric[]): string {
  const header =
    `| File | Direct Dependents | Total Blast Radius | Bottleneck Risk Score |\n` +
    `| :--- | :---: | :---: | :---: |\n`;
  return header + touchedBottlenecks.map(findingsRow).join('');
}

function findingsRow(metric: ModuleMetric): string {
  return `| \`${metric.file}\` | ${metric.directCount} | ${metric.blastRadius} modules | **${metric.score}** (${riskBadge(metric.score)}) |\n`;
}

function riskBadge(score: number): string {
  return score > HIGH_RISK_SCORE ? '🔴 High Risk' : '🟡 Moderate Risk';
}

function analyzedFilesLine(analyzedCount: number): string {
  const fileWord = analyzedCount === 1 ? 'file' : 'files';
  return `Analyzed ${analyzedCount} changed \`src/\` ${fileWord}.`;
}
