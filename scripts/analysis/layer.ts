/**
 * Classifies a first-party `src/` module into a hexagonal architecture layer so
 * the change-impact graph can group and colour nodes by their role in the
 * layered hexagonal architecture (see backend-architecture steering).
 *
 * The visible buckets mirror the target folder structure:
 *   - `domain/`      → pure core (entities, value objects, ports)
 *   - `application/` → use cases orchestrating the domain
 *   - `infra/`       → adapters implementing domain ports
 *   - everything else in `src/` (handlers, services, utils, types) → legacy
 */
export enum Layer {
  Domain = 'domain',
  Application = 'application',
  Infra = 'infra',
  Legacy = 'legacy',
}

export interface LayerStyle {
  /** Human-readable subgraph title. */
  readonly title: string;
  /** Muted fill colour (not bright) for the layer's classDef. */
  readonly fill: string;
  /** Slightly darker stroke to keep nodes legible against the fill. */
  readonly stroke: string;
  /**
   * Hexagonal rank: lower ranks render first (outermost adapters), higher
   * ranks render last (innermost core). Ordering subgraphs this way makes the
   * dependency flow read inward toward the domain.
   */
  readonly rank: number;
}

const LAYER_STYLES: Readonly<Record<Layer, LayerStyle>> = {
  [Layer.Legacy]: {
    title: 'Legacy',
    fill: '#ece6de',
    stroke: '#b9ac99',
    rank: 0,
  },
  [Layer.Infra]: {
    title: 'Infrastructure',
    fill: '#dfe6ec',
    stroke: '#9fb3c4',
    rank: 1,
  },
  [Layer.Application]: {
    title: 'Application',
    fill: '#e2e6dc',
    stroke: '#a9b79a',
    rank: 2,
  },
  [Layer.Domain]: {
    title: 'Domain',
    fill: '#e6e0ec',
    stroke: '#b3a3c4',
    rank: 3,
  },
};

export function classifyLayer(filePath: string): Layer {
  if (filePath.startsWith('src/domain/')) return Layer.Domain;
  if (filePath.startsWith('src/application/')) return Layer.Application;
  if (filePath.startsWith('src/infra/')) return Layer.Infra;
  return Layer.Legacy;
}

export function layerStyle(layer: Layer): LayerStyle {
  return LAYER_STYLES[layer];
}

const FOLDER_GROUPED_LAYERS: ReadonlySet<Layer> = new Set([Layer.Domain, Layer.Infra]);

export function usesFolderGrouping(layer: Layer): boolean {
  return FOLDER_GROUPED_LAYERS.has(layer);
}

/** Layers ordered from outermost adapter (legacy) to innermost core (domain). */
export function layersInHexagonalOrder(): Layer[] {
  return Object.values(Layer).sort((a, b) => LAYER_STYLES[a].rank - LAYER_STYLES[b].rank);
}
