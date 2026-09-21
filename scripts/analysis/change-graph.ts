import { CHANGE_GRAPH_MAX_NODES } from './config.ts';
import type { DependencyGraph, DependencyModule } from './types.ts';

const NODE_MODULES_PATTERN = /(^|\/)node_modules\//;
const TEST_FILE_PATTERN = /\.(test|spec)\.[cm]?[jt]sx?$/;

export interface ChangeGraphEdge {
  from: string;
  to: string;
}

export interface ChangeGraph {
  changedFiles: Set<string>;
  neighbors: Set<string>;
  edges: ChangeGraphEdge[];
  truncated: boolean;
}

/**
 * Builds the review subgraph: the changed files plus their immediate
 * dependencies (files they import) and immediate dependents (files that import
 * them). Only first-party `src/` modules are included; edges point from
 * importer to imported.
 */
export function buildChangeGraph(graph: DependencyGraph, changedFiles: string[]): ChangeGraph {
  const changed = new Set(changedFiles.filter(isFirstPartyPath));
  const moduleBySource = indexBySource(graph);

  const edges = collectImmediateEdges(changed, moduleBySource);
  const neighbors = collectNeighbors(edges, changed);

  return capNodes({ changedFiles: changed, neighbors, edges, truncated: false });
}

function indexBySource(graph: DependencyGraph): Map<string, DependencyModule> {
  const index = new Map<string, DependencyModule>();
  for (const module of graph.modules) {
    index.set(module.source, module);
  }
  return index;
}

function collectImmediateEdges(
  changed: Set<string>,
  moduleBySource: Map<string, DependencyModule>,
): ChangeGraphEdge[] {
  const edges = new Map<string, ChangeGraphEdge>();

  for (const file of changed) {
    const module = moduleBySource.get(file);
    if (!module) continue;
    addDependencyEdges(file, module, edges);
    addDependentEdges(file, module, edges);
  }

  return [...edges.values()];
}

function addDependencyEdges(
  file: string,
  module: DependencyModule,
  edges: Map<string, ChangeGraphEdge>,
): void {
  for (const dependency of module.dependencies ?? []) {
    if (dependency.coreModule || !isFirstPartyPath(dependency.resolved)) continue;
    recordEdge(edges, file, dependency.resolved);
  }
}

function addDependentEdges(
  file: string,
  module: DependencyModule,
  edges: Map<string, ChangeGraphEdge>,
): void {
  for (const dependent of module.dependents ?? []) {
    if (!isFirstPartyPath(dependent)) continue;
    recordEdge(edges, dependent, file);
  }
}

function recordEdge(edges: Map<string, ChangeGraphEdge>, from: string, to: string): void {
  if (from === to) return;
  edges.set(`${from}\u0000${to}`, { from, to });
}

function collectNeighbors(edges: ChangeGraphEdge[], changed: Set<string>): Set<string> {
  const neighbors = new Set<string>();
  for (const edge of edges) {
    addIfNeighbor(neighbors, edge.from, changed);
    addIfNeighbor(neighbors, edge.to, changed);
  }
  return neighbors;
}

function addIfNeighbor(neighbors: Set<string>, file: string, changed: Set<string>): void {
  if (!changed.has(file)) neighbors.add(file);
}

/**
 * Caps the graph to a node budget so a large PR cannot produce an unreadable
 * Mermaid diagram. Changed files are always kept; neighbors are dropped first
 * (along with any edge that references a dropped node).
 */
function capNodes(graph: ChangeGraph): ChangeGraph {
  const budget = CHANGE_GRAPH_MAX_NODES - graph.changedFiles.size;
  if (graph.neighbors.size <= budget) return graph;

  const keptNeighbors = new Set([...graph.neighbors].slice(0, Math.max(budget, 0)));
  const visible = new Set([...graph.changedFiles, ...keptNeighbors]);
  const edges = graph.edges.filter(edge => visible.has(edge.from) && visible.has(edge.to));

  return { changedFiles: graph.changedFiles, neighbors: keptNeighbors, edges, truncated: true };
}

function isFirstPartyPath(filePath: string): boolean {
  return (
    filePath.startsWith('src/') &&
    !NODE_MODULES_PATTERN.test(filePath) &&
    !TEST_FILE_PATTERN.test(filePath)
  );
}
