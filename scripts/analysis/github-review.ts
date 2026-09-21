import { PER_PAGE, request } from '../shared/github-client.ts';
import type { PullRequestContext } from '../shared/types.ts';
import type { AddedComment, ExistingReviewComment } from './types.ts';

export { readPullRequestContext } from '../shared/pull-request-context.ts';

export async function fetchExistingComments(
  context: PullRequestContext,
): Promise<ExistingReviewComment[]> {
  const comments: ExistingReviewComment[] = [];
  for (let page = 1; ; page += 1) {
    const path = `/repos/${context.owner}/${context.repo}/pulls/${context.pullNumber}/comments?per_page=${PER_PAGE}&page=${page}`;
    const batch = (await request(context, 'GET', path)) as ExistingReviewComment[];
    comments.push(...batch);
    if (batch.length < PER_PAGE) break;
  }
  return comments;
}

export async function createReviewComment(
  context: PullRequestContext,
  comment: AddedComment,
  body: string,
): Promise<void> {
  const path = `/repos/${context.owner}/${context.repo}/pulls/${context.pullNumber}/comments`;
  const isMultiLine = comment.endLine > comment.startLine;
  await request(context, 'POST', path, {
    body,
    commit_id: context.headSha,
    path: comment.file,
    line: comment.endLine,
    side: 'RIGHT',
    start_line: isMultiLine ? comment.startLine : undefined,
    start_side: isMultiLine ? 'RIGHT' : undefined,
  });
}
