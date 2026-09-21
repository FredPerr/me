export const COMMENT_MARKER = '<!-- loadup-changelog -->';

export enum CommitType {
  Feature = 'feat',
  Fix = 'fix',
  Performance = 'perf',
  Refactor = 'refactor',
  Build = 'build',
  Ci = 'ci',
  Docs = 'docs',
  Test = 'test',
  Style = 'style',
  Chore = 'chore',
}

/** Ordered section definitions; groups without commits are omitted from output. */
export const SECTION_TITLES: ReadonlyArray<readonly [CommitType, string]> = [
  [CommitType.Feature, 'Features'],
  [CommitType.Fix, 'Fixes'],
  [CommitType.Performance, 'Performance'],
  [CommitType.Refactor, 'Refactoring'],
  [CommitType.Build, 'Build'],
  [CommitType.Ci, 'CI'],
  [CommitType.Docs, 'Documentation'],
  [CommitType.Test, 'Tests'],
  [CommitType.Style, 'Style'],
  [CommitType.Chore, 'Chores'],
];

export const OTHER_SECTION_TITLE = 'Other';
