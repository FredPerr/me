import { readPullRequestContext } from '../shared/pull-request-context.ts';
import { upsertIssueComment } from '../shared/issue-comments.ts';
import { renderChangelogComment } from './changelog-builder.ts';
import { groupCommits } from './commit-parser.ts';
import { COMMENT_MARKER } from './config.ts';
import { fetchPullRequestCommits } from './pull-request-commits.ts';

async function main(): Promise<void> {
  const context = readPullRequestContext();
  if (!context) {
    console.log('Not running in a pull request context; skipping changelog comment.');
    return;
  }

  const commits = await fetchPullRequestCommits(context);
  const sections = groupCommits(commits);
  const body = renderChangelogComment({
    targetBranch: context.baseRef,
    sections,
  });

  await upsertIssueComment(context, COMMENT_MARKER, body);
  console.log(`Upserted changelog comment for PR #${context.pullNumber}.`);
}

main().catch(error => {
  console.error(`Error: ${error instanceof Error ? error.message : String(error)}`);
  process.exit(1);
});
