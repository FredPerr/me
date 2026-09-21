import { describe, it, expect } from 'vitest';
import { buildChangeGraph } from './change-graph.ts';
import type { DependencyGraph, DependencyModule } from './types.ts';

const SOURCE_FILE = 'src/domain/projects/Project.ts';
const NEIGHBOR_FILE = 'src/infra/persistence/project.repository.ts';
const TEST_NEIGHBOR_FILE = 'src/domain/projects/Project.test.ts';

function buildModule(overrides: Partial<DependencyModule> & { source: string }): DependencyModule {
  return {
    dependencies: [],
    dependents: [],
    ...overrides,
  };
}

function buildGraph(modules: DependencyModule[]): DependencyGraph {
  return { modules };
}

describe('buildChangeGraph', () => {
  it('includes first-party dependency and dependent neighbors', () => {
    const graph = buildGraph([
      buildModule({
        source: SOURCE_FILE,
        dependencies: [{ resolved: NEIGHBOR_FILE }],
      }),
    ]);

    const result = buildChangeGraph(graph, [SOURCE_FILE]);

    expect(result.neighbors.has(NEIGHBOR_FILE)).toBe(true);
    expect(result.edges).toContainEqual({ from: SOURCE_FILE, to: NEIGHBOR_FILE });
  });

  it('excludes test files that depend on a changed file', () => {
    const graph = buildGraph([
      buildModule({
        source: SOURCE_FILE,
        dependents: [TEST_NEIGHBOR_FILE],
      }),
    ]);

    const result = buildChangeGraph(graph, [SOURCE_FILE]);

    expect(result.neighbors.has(TEST_NEIGHBOR_FILE)).toBe(false);
    expect(result.edges).toHaveLength(0);
  });

  it('excludes a changed test file from the graph', () => {
    const graph = buildGraph([
      buildModule({
        source: TEST_NEIGHBOR_FILE,
        dependencies: [{ resolved: SOURCE_FILE }],
      }),
    ]);

    const result = buildChangeGraph(graph, [TEST_NEIGHBOR_FILE]);

    expect(result.changedFiles.has(TEST_NEIGHBOR_FILE)).toBe(false);
    expect(result.edges).toHaveLength(0);
  });
});
