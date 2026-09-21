import {
  BLAST_RADIUS_WEIGHT,
  DIRECT_DEPENDENT_WEIGHT,
  MAX_SCORE,
} from './config.ts';
import type {
  DependencyGraph,
  DependencyModule,
  DirectDependents,
  ModuleMetric,
} from './types.ts';

const NODE_MODULES_PATTERN = /(^|\/)node_modules\//;

export function computeMetrics(graph: DependencyGraph): ModuleMetric[] {
  const directDependents = buildDirectDependents(graph);
  return graph.modules
    .filter(isFirstPartyModule)
    .map(module => scoreModule(module, directDependents))
    .sort(worstFirst);
}

function buildDirectDependents(graph: DependencyGraph): DirectDependents {
  const directDependents: DirectDependents = {};
  for (const module of graph.modules) {
    directDependents[module.source] = module.dependents ?? [];
  }
  return directDependents;
}

function isFirstPartyModule(module: DependencyModule): boolean {
  return !module.coreModule && !NODE_MODULES_PATTERN.test(module.source);
}

function scoreModule(
  module: DependencyModule,
  directDependents: DirectDependents,
): ModuleMetric {
  const file = module.source;
  const directCount = (directDependents[file] ?? []).length;
  const blastRadius = computeBlastRadius(file, directDependents);
  return { file, directCount, blastRadius, score: riskScore(directCount, blastRadius) };
}

function computeBlastRadius(file: string, directDependents: DirectDependents): number {
  const visited = new Set<string>();
  const queue: string[] = [file];

  while (queue.length > 0) {
    const current = queue.shift() as string;
    for (const dependent of directDependents[current] ?? []) {
      if (!visited.has(dependent) && dependent !== file) {
        visited.add(dependent);
        queue.push(dependent);
      }
    }
  }
  return visited.size;
}

function riskScore(directCount: number, blastRadius: number): number {
  const raw = directCount * DIRECT_DEPENDENT_WEIGHT + blastRadius * BLAST_RADIUS_WEIGHT;
  return Math.min(MAX_SCORE, raw);
}

function worstFirst(a: ModuleMetric, b: ModuleMetric): number {
  return (
    b.score - a.score ||
    b.blastRadius - a.blastRadius ||
    b.directCount - a.directCount
  );
}
