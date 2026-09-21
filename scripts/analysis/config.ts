import path from 'node:path';

export const RISK_SCORE_THRESHOLD = 66;
export const MIN_FILES_TO_COMMENT = 1;
export const HIGH_RISK_SCORE = 85;
export const MAX_SCORE = 100;
export const DIRECT_DEPENDENT_WEIGHT = 5;
export const BLAST_RADIUS_WEIGHT = 2;
export const LOCAL_REPORT_LIMIT = 200;
export const PATH_TRUNCATE_LENGTH = 60;

export const COMMENT_MARKER = '<!-- bottleneck-analysis-comment -->';

export const CHANGE_GRAPH_MARKER = '<!-- change-graph-comment -->';
export const CHANGE_GRAPH_MAX_NODES = 60;

export const COMMENT_NUDGE_MARKER_PREFIX = '<!-- comment-nudge:';
export const COMMENT_NUDGE_BODY =
  'If this code is unclear, consider making it clearer and removing the comment. ' +
  'If the comment carries real intent (a non-obvious *why*), keep it. Resolve this thread once addressed.';

export const ENUM_NUDGE_MARKER_PREFIX = '<!-- enum-nudge:';
export const ENUM_NUDGE_BODY =
  'This is a string-literal union type. Per our conventions, prefer an `enum` for a fixed set of named values. ' +
  'If the values are truly open-ended or better modeled as a union, keep it. Resolve this thread once addressed.';

export const DEP_GRAPH_FILENAME = 'dep-graph.json';
export const COMMENT_FILENAME = 'bottleneck-comment.md';
export const IGNORE_FILENAME = 'ignore.txt';

export const DEPCRUISE_COMMAND =
  'npx depcruise src --config .dependency-cruiser.cjs --output-type json --output-to dep-graph.json';

export function depGraphPath(): string {
  return path.resolve(process.cwd(), DEP_GRAPH_FILENAME);
}

export function commentPath(): string {
  return path.resolve(process.cwd(), COMMENT_FILENAME);
}

export function ignoreListPath(scriptDirectory: string): string {
  return path.resolve(scriptDirectory, IGNORE_FILENAME);
}
