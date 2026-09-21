import { PER_PAGE, request } from '../shared/github-client.ts';
import type { PullRequestContext } from '../shared/types.ts';
import type { PullRequestCommit } from './types.ts';

interface RawCommit {
  sha: string;
  commit: { message: string };
}

export async function fetchPullRequestCommits(
  context: PullRequestContext,
): Promise<PullRequestCommit[]> {
  const commits: PullRequestCommit[] = [];
  for (let page = 1; ; page += 1) {
    const path = `/repos/${context.owner}/${context.repo}/pulls/${context.pullNumber}/commits?per_page=${PER_PAGE}&page=${page}`;
    const batch = (await request(context, 'GET', path)) as RawCommit[];
    for (const raw of batch) {
      commits.push({ sha: raw.sha, subject: firstLine(raw.commit.message) });
    }
    if (batch.length < PER_PAGE) break;
  }
  return commits;
}

function firstLine(message: string): string {
  return message.split('\n')[0].trim();
}
