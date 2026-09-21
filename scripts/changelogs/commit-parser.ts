import {
  CommitType,
  OTHER_SECTION_TITLE,
  SECTION_TITLES,
} from './config.ts';
import type { ChangelogEntry, ChangelogSection, PullRequestCommit } from './types.ts';

const SHORT_SHA_LENGTH = 7;
const CONVENTIONAL_PATTERN =
  /^(feat|fix|perf|refactor|build|ci|docs|test|style|chore)(\([^)]+\))?(!)?:\s*(.+)$/;

const COMMIT_TYPES = new Set<string>(Object.values(CommitType));

export function groupCommits(commits: PullRequestCommit[]): ChangelogSection[] {
  const entriesByType = new Map<string, ChangelogEntry[]>();
  const otherEntries: ChangelogEntry[] = [];

  for (const commit of commits) {
    if (shouldSkip(commit.subject)) continue;

    const entry = parseCommit(commit);
    const bucket = entry.type ? entriesByType.get(entry.type.type) : undefined;
    if (entry.type && bucket) {
      bucket.push(entry.entry);
    } else if (entry.type) {
      entriesByType.set(entry.type.type, [entry.entry]);
    } else {
      otherEntries.push(entry.entry);
    }
  }

  return buildSections(entriesByType, otherEntries);
}

interface ParsedCommit {
  type: { type: string } | null;
  entry: ChangelogEntry;
}

function parseCommit(commit: PullRequestCommit): ParsedCommit {
  const shortSha = commit.sha.substring(0, SHORT_SHA_LENGTH);
  const match = commit.subject.match(CONVENTIONAL_PATTERN);

  if (!match) {
    return {
      type: null,
      entry: { description: commit.subject, scope: null, isBreaking: false, shortSha },
    };
  }

  const [, type, rawScope, breaking, description] = match;
  return {
    type: COMMIT_TYPES.has(type) ? { type } : null,
    entry: {
      description,
      scope: rawScope ? rawScope.slice(1, -1) : null,
      isBreaking: Boolean(breaking),
      shortSha,
    },
  };
}

function buildSections(
  entriesByType: Map<string, ChangelogEntry[]>,
  otherEntries: ChangelogEntry[],
): ChangelogSection[] {
  const sections: ChangelogSection[] = [];

  for (const [type, title] of SECTION_TITLES) {
    const entries = entriesByType.get(type);
    if (entries && entries.length > 0) {
      sections.push({ title, entries });
    }
  }

  if (otherEntries.length > 0) {
    sections.push({ title: OTHER_SECTION_TITLE, entries: otherEntries });
  }

  return sections;
}

function shouldSkip(subject: string): boolean {
  return subject.startsWith('Merge ') || subject.includes('[skip ci]');
}
