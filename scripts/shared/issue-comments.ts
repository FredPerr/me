import { PER_PAGE, request } from './github-client.ts';
import type { IssueComment, PullRequestContext } from './types.ts';

export async function fetchIssueComments(
  context: PullRequestContext,
): Promise<IssueComment[]> {
  const comments: IssueComment[] = [];
  for (let page = 1; ; page += 1) {
    const path = `/repos/${context.owner}/${context.repo}/issues/${context.pullNumber}/comments?per_page=${PER_PAGE}&page=${page}`;
    const batch = (await request(context, 'GET', path)) as IssueComment[];
    comments.push(...batch);
    if (batch.length < PER_PAGE) break;
  }
  return comments;
}

/**
 * Creates a comment on the pull request, or updates the existing one whose body
 * starts with `marker`. The marker keeps a single sticky comment in place across
 * pushes instead of posting a new one each time.
 */
export async function upsertIssueComment(
  context: PullRequestContext,
  marker: string,
  body: string,
): Promise<void> {
  const existing = (await fetchIssueComments(context)).find(comment =>
    comment.body?.startsWith(marker),
  );

  if (existing) {
    await request(
      context,
      'PATCH',
      `/repos/${context.owner}/${context.repo}/issues/comments/${existing.id}`,
      { body },
    );
    return;
  }

  await request(
    context,
    'POST',
    `/repos/${context.owner}/${context.repo}/issues/${context.pullNumber}/comments`,
    { body },
  );
}
