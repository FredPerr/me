export interface ModuleDependency {
  resolved: string;
  coreModule?: boolean;
}

export interface DependencyModule {
  source: string;
  dependencies?: ModuleDependency[];
  dependents?: string[];
  coreModule?: boolean;
}

export interface DependencyGraph {
  modules: DependencyModule[];
}

export interface ModuleMetric {
  file: string;
  directCount: number;
  blastRadius: number;
  score: number;
}

export interface CliArgs {
  isLocal: boolean;
  changedFiles: string[];
}

export type DirectDependents = Record<string, string[]>;

export interface AddedComment {
  file: string;
  startLine: number;
  endLine: number;
}

export interface AddedUnionType {
  file: string;
  startLine: number;
  endLine: number;
  name: string;
}

export type { PullRequestContext } from '../shared/types.ts';

export interface ExistingReviewComment {
  path: string;
  line: number | null;
  body: string;
}
