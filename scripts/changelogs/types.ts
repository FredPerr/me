export interface PullRequestCommit {
  sha: string;
  subject: string;
}

export interface ChangelogEntry {
  description: string;
  scope: string | null;
  isBreaking: boolean;
  shortSha: string;
}

export interface ChangelogSection {
  title: string;
  entries: ChangelogEntry[];
}

export interface Changelog {
  targetBranch: string;
  sections: ChangelogSection[];
}
