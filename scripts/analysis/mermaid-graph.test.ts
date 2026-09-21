import { describe, it, expect } from 'vitest';
import { renderMermaid } from './mermaid-graph.ts';
import type { ChangeGraph, ChangeGraphEdge } from './change-graph.ts';

const DOMAIN_FILE = 'src/domain/projects/Project.ts';
const DOMAIN_SIBLING_FILE = 'src/domain/projects/ProjectStatus.ts';
const INFRA_FILE = 'src/infra/persistence/project.repository.ts';
const LEGACY_FILE = 'src/handlers/projects/create.ts';

interface ChangeGraphOverrides {
  changedFiles?: string[];
  neighbors?: string[];
  edges?: ChangeGraphEdge[];
  truncated?: boolean;
}

function buildChangeGraph(overrides?: ChangeGraphOverrides): ChangeGraph {
  return {
    changedFiles: new Set(overrides?.changedFiles ?? []),
    neighbors: new Set(overrides?.neighbors ?? []),
    edges: overrides?.edges ?? [],
    truncated: overrides?.truncated ?? false,
  };
}

describe('renderMermaid', () => {
  it('wraps the output in a mermaid code fence with a flowchart', () => {
    const graph = buildChangeGraph({ changedFiles: [DOMAIN_FILE] });

    const output = renderMermaid(graph);

    expect(output.startsWith('```mermaid')).toBe(true);
    expect(output).toContain('graph LR');
    expect(output.trimEnd().endsWith('```')).toBe(true);
  });

  it('groups each layer into its own titled subgraph', () => {
    const graph = buildChangeGraph({
      changedFiles: [DOMAIN_FILE],
      neighbors: [INFRA_FILE, LEGACY_FILE],
    });

    const output = renderMermaid(graph);

    expect(output).toContain('subgraph layer_domain["Domain"]');
    expect(output).toContain('subgraph layer_infra["Infrastructure"]');
    expect(output).toContain('subgraph layer_legacy["Legacy"]');
  });

  it('orders subgraphs from legacy (outer) to domain (core)', () => {
    const graph = buildChangeGraph({
      changedFiles: [DOMAIN_FILE],
      neighbors: [INFRA_FILE, LEGACY_FILE],
    });

    const output = renderMermaid(graph);

    const legacyIndex = output.indexOf('subgraph layer_legacy');
    const infraIndex = output.indexOf('subgraph layer_infra');
    const domainIndex = output.indexOf('subgraph layer_domain');
    expect(legacyIndex).toBeLessThan(infraIndex);
    expect(infraIndex).toBeLessThan(domainIndex);
  });

  it('assigns a muted layer class to each rendered layer', () => {
    const graph = buildChangeGraph({ neighbors: [INFRA_FILE, LEGACY_FILE] });

    const output = renderMermaid(graph);

    expect(output).toContain('classDef infra fill:');
    expect(output).toContain('classDef legacy fill:');
  });

  it('highlights changed files after their layer class so the highlight wins', () => {
    const graph = buildChangeGraph({ changedFiles: [DOMAIN_FILE] });

    const output = renderMermaid(graph);

    const layerClassIndex = output.indexOf('classDef domain fill:');
    const changedClassIndex = output.indexOf('classDef changed fill:');
    expect(layerClassIndex).toBeLessThan(changedClassIndex);
  });

  it('renders an edge from importer to imported', () => {
    const graph = buildChangeGraph({
      changedFiles: [INFRA_FILE],
      neighbors: [DOMAIN_FILE],
      edges: [{ from: INFRA_FILE, to: DOMAIN_FILE }],
    });

    const output = renderMermaid(graph);

    expect(output).toContain('n0 --> n1');
  });

  it('omits layers that have no files', () => {
    const graph = buildChangeGraph({ changedFiles: [DOMAIN_FILE] });

    const output = renderMermaid(graph);

    expect(output).not.toContain('subgraph layer_infra');
    expect(output).not.toContain('subgraph layer_legacy');
  });

  it('nests a folder subgraph inside domain and infra layers', () => {
    const graph = buildChangeGraph({
      changedFiles: [DOMAIN_FILE],
      neighbors: [INFRA_FILE],
    });

    const output = renderMermaid(graph);

    expect(output).toContain('subgraph dir_src_domain_projects["src/domain/projects"]');
    expect(output).toContain('subgraph dir_src_infra_persistence["src/infra/persistence"]');
  });

  it('labels folder-grouped nodes with only the file name', () => {
    const graph = buildChangeGraph({ changedFiles: [DOMAIN_FILE] });

    const output = renderMermaid(graph);

    expect(output).toContain('["Project.ts"]');
    expect(output).not.toContain('["src/domain/projects/Project.ts"]');
  });

  it('places same-folder files under a single folder subgraph', () => {
    const graph = buildChangeGraph({
      changedFiles: [DOMAIN_FILE],
      neighbors: [DOMAIN_SIBLING_FILE],
    });

    const output = renderMermaid(graph);

    const folderSubgraphCount = output.split('subgraph dir_src_domain_projects').length - 1;
    expect(folderSubgraphCount).toBe(1);
    expect(output).toContain('["Project.ts"]');
    expect(output).toContain('["ProjectStatus.ts"]');
  });

  it('keeps legacy nodes flat with a path label instead of folder subgraphs', () => {
    const graph = buildChangeGraph({ changedFiles: [LEGACY_FILE] });

    const output = renderMermaid(graph);

    expect(output).not.toContain('subgraph dir_');
    expect(output).toContain('["src/handlers/projects/create.ts"]');
  });
});
