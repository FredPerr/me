import { CHANGE_GRAPH_MARKER } from './config.ts';
import type { ChangeGraph } from './change-graph.ts';
import { renderMermaid } from './mermaid-graph.ts';

export function buildChangeGraphComment(graph: ChangeGraph): string {
  return [
    CHANGE_GRAPH_MARKER,
    '### 🕸️ Change Impact Graph',
    '',
    summaryLine(graph),
    '',
    'Highlighted nodes are changed files; connected nodes are their immediate dependencies and dependents. Arrows point from importer to imported.',
    '',
    renderMermaid(graph),
    truncationNote(graph),
  ]
    .filter(line => line !== null)
    .join('\n');
}

export function buildEmptyChangeGraphComment(): string {
  return [
    CHANGE_GRAPH_MARKER,
    '### 🕸️ Change Impact Graph',
    '',
    'No first-party `src/` modules with tracked dependencies were changed in this PR.',
  ].join('\n');
}

function summaryLine(graph: ChangeGraph): string {
  const changedWord = graph.changedFiles.size === 1 ? 'file' : 'files';
  const neighborWord = graph.neighbors.size === 1 ? 'neighbor' : 'neighbors';
  return `Showing ${graph.changedFiles.size} changed ${changedWord} and ${graph.neighbors.size} immediate ${neighborWord}.`;
}

function truncationNote(graph: ChangeGraph): string | null {
  if (!graph.truncated) return null;
  return '\n> ⚠️ The graph was capped to keep it readable. Some neighbors are not shown.';
}
