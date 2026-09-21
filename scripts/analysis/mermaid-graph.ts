import { truncateMiddle } from './format-path.ts';
import type { ChangeGraph } from './change-graph.ts';
import { Layer, classifyLayer, layerStyle, layersInHexagonalOrder, usesFolderGrouping } from './layer.ts';

const CHANGED_CLASS = 'changed';
const LABEL_MAX_LENGTH = 42;
const FILE_NAME_MAX_LENGTH = 32;


export function renderMermaid(graph: ChangeGraph): string {
  const files = [...graph.changedFiles, ...graph.neighbors];
  const nodeIds = assignNodeIds(files);
  const lines = [
    '```mermaid',
    'graph LR',
    ...layerSubgraphs(files, nodeIds),
    ...edgeDeclarations(graph, nodeIds),
    ...layerClassDefinitions(files, nodeIds),
    ...changedClassDefinition(graph, nodeIds),
    '```',
  ];
  return lines.join('\n');
}

function assignNodeIds(files: string[]): Map<string, string> {
  const nodeIds = new Map<string, string>();
  files.forEach((file, index) => nodeIds.set(file, `n${index}`));
  return nodeIds;
}

function layerSubgraphs(files: string[], nodeIds: Map<string, string>): string[] {
  const filesByLayer = groupByLayer(files);
  const lines: string[] = [];

  for (const layer of layersInHexagonalOrder()) {
    const layerFiles = filesByLayer.get(layer);
    if (!layerFiles || layerFiles.length === 0) continue;
    lines.push(`  subgraph ${layerSubgraphId(layer)}["${layerStyle(layer).title}"]`);
    lines.push(...layerBody(layer, layerFiles, nodeIds));
    lines.push('  end');
  }

  return lines;
}

function layerBody(layer: Layer, files: string[], nodeIds: Map<string, string>): string[] {
  return usesFolderGrouping(layer)
    ? folderGroupedNodes(files, nodeIds)
    : files.map(file => flatNode(file, nodeIds));
}

function folderGroupedNodes(files: string[], nodeIds: Map<string, string>): string[] {
  const lines: string[] = [];
  for (const [folder, folderFiles] of groupByFolder(files)) {
    lines.push(`    subgraph ${folderSubgraphId(folder)}["${escapeLabel(folder)}"]`);
    for (const file of folderFiles) {
      lines.push(`      ${nodeIds.get(file)}["${escapeLabel(fileName(file))}"]`);
    }
    lines.push('    end');
  }
  return lines;
}

function flatNode(file: string, nodeIds: Map<string, string>): string {
  return `    ${nodeIds.get(file)}["${nodeLabel(file)}"]`;
}

function groupByLayer(files: string[]): Map<Layer, string[]> {
  const grouped = new Map<Layer, string[]>();
  for (const file of files) {
    const layer = classifyLayer(file);
    const bucket = grouped.get(layer) ?? [];
    bucket.push(file);
    grouped.set(layer, bucket);
  }
  return grouped;
}

function groupByFolder(files: string[]): Map<string, string[]> {
  const grouped = new Map<string, string[]>();
  for (const file of files) {
    const folder = parentFolder(file);
    const bucket = grouped.get(folder) ?? [];
    bucket.push(file);
    grouped.set(folder, bucket);
  }
  return grouped;
}

function edgeDeclarations(graph: ChangeGraph, nodeIds: Map<string, string>): string[] {
  return graph.edges.map(edge => `  ${nodeIds.get(edge.from)} --> ${nodeIds.get(edge.to)}`);
}

function layerClassDefinitions(files: string[], nodeIds: Map<string, string>): string[] {
  const filesByLayer = groupByLayer(files);
  const lines: string[] = [];

  for (const layer of layersInHexagonalOrder()) {
    const layerFiles = filesByLayer.get(layer);
    if (!layerFiles || layerFiles.length === 0) continue;
    const style = layerStyle(layer);
    const ids = layerFiles.map(file => nodeIds.get(file)).join(',');
    lines.push(`  classDef ${layer} fill:${style.fill},stroke:${style.stroke},color:#000;`);
    lines.push(`  class ${ids} ${layer};`);
  }

  return lines;
}

function changedClassDefinition(graph: ChangeGraph, nodeIds: Map<string, string>): string[] {
  if (graph.changedFiles.size === 0) return [];
  const changedIds = [...graph.changedFiles].map(file => nodeIds.get(file)).join(',');
  return [
    `  classDef ${CHANGED_CLASS} fill:#ffd27f,stroke:#d98c00,stroke-width:2px,color:#000;`,
    `  class ${changedIds} ${CHANGED_CLASS};`,
  ];
}

function layerSubgraphId(layer: Layer): string {
  return `layer_${layer}`;
}

function folderSubgraphId(folder: string): string {
  return `dir_${folder.replace(/[^a-zA-Z0-9]/g, '_')}`;
}

function parentFolder(file: string): string {
  const lastSlash = file.lastIndexOf('/');
  return lastSlash === -1 ? file : file.slice(0, lastSlash);
}

function fileName(file: string): string {
  const name = file.slice(file.lastIndexOf('/') + 1);
  return truncateMiddle(name, FILE_NAME_MAX_LENGTH);
}

function nodeLabel(file: string): string {
  return escapeLabel(truncateMiddle(file, LABEL_MAX_LENGTH));
}

function escapeLabel(label: string): string {
  return label.replace(/"/g, '&quot;');
}
