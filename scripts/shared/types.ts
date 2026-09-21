export interface PullRequestContext {
  owner: string;
  repo: string;
  pullNumber: number;
  headSha: string;
  baseRef: string;
  token: string;
}

export interface IssueComment {
  id: number;
  body: string;
}
