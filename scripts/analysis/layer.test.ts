import { describe, it, expect } from 'vitest';
import {
  Layer,
  classifyLayer,
  layerStyle,
  layersInHexagonalOrder,
  usesFolderGrouping,
} from './layer.ts';

const DOMAIN_FILE = 'src/domain/projects/Project.ts';
const APPLICATION_FILE = 'src/application/projects/CreateProject.ts';
const INFRA_FILE = 'src/infra/persistence/project.repository.ts';
const LEGACY_HANDLER_FILE = 'src/handlers/projects/create.ts';
const LEGACY_UTIL_FILE = 'src/utils/logger.ts';

describe('classifyLayer', () => {
  it('classifies files under src/domain as the domain layer', () => {
    expect(classifyLayer(DOMAIN_FILE)).toBe(Layer.Domain);
  });

  it('classifies files under src/application as the application layer', () => {
    expect(classifyLayer(APPLICATION_FILE)).toBe(Layer.Application);
  });

  it('classifies files under src/infra as the infra layer', () => {
    expect(classifyLayer(INFRA_FILE)).toBe(Layer.Infra);
  });

  it('classifies handlers outside the hexagonal folders as legacy', () => {
    expect(classifyLayer(LEGACY_HANDLER_FILE)).toBe(Layer.Legacy);
  });

  it('classifies shared utils as legacy', () => {
    expect(classifyLayer(LEGACY_UTIL_FILE)).toBe(Layer.Legacy);
  });
});

describe('layersInHexagonalOrder', () => {
  it('orders layers from outermost adapter to innermost core', () => {
    expect(layersInHexagonalOrder()).toEqual([
      Layer.Legacy,
      Layer.Infra,
      Layer.Application,
      Layer.Domain,
    ]);
  });
});

describe('layerStyle', () => {
  it('gives every layer a distinct muted fill colour', () => {
    const fills = layersInHexagonalOrder().map(layer => layerStyle(layer).fill);

    expect(new Set(fills).size).toBe(fills.length);
  });

  it('exposes a human-readable title per layer', () => {
    expect(layerStyle(Layer.Domain).title).toBe('Domain');
    expect(layerStyle(Layer.Legacy).title).toBe('Legacy');
  });
});

describe('usesFolderGrouping', () => {
  it('groups the domain and infra layers by folder', () => {
    expect(usesFolderGrouping(Layer.Domain)).toBe(true);
    expect(usesFolderGrouping(Layer.Infra)).toBe(true);
  });

  it('does not group the legacy layer by folder', () => {
    expect(usesFolderGrouping(Layer.Legacy)).toBe(false);
  });
});
