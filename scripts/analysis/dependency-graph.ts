import fs from 'node:fs';
import { execSync } from 'node:child_process';
import { DEPCRUISE_COMMAND, depGraphPath } from './config.ts';
import type { DependencyGraph } from './types.ts';

export function loadDependencyGraph(): DependencyGraph {
  const graphPath = depGraphPath();
  ensureGraphExists(graphPath);
  return JSON.parse(fs.readFileSync(graphPath, 'utf8')) as DependencyGraph;
}

function ensureGraphExists(graphPath: string): void {
  if (fs.existsSync(graphPath)) return;
  generateGraph();
  if (!fs.existsSync(graphPath)) {
    throw new Error('Could not find or generate dependency graph.');
  }
}

function generateGraph(): void {
  execSync(DEPCRUISE_COMMAND, { stdio: 'ignore' });
}
