import { COMMENT_NUDGE_BODY, COMMENT_NUDGE_MARKER_PREFIX } from './config.ts';
import { findAddedComments } from './diff-comments.ts';
import {
  createReviewComment,
  fetchExistingComments,
  readPullRequestContext,
} from './github-review.ts';
import type { AddedComment, ExistingReviewComment, PullRequestContext } from './types.ts';

async function main(): Promise<void> {
  const context = readPullRequestContext();
  if (!context) {
    console.log('Not running in a pull request context; skipping comment nudges.');
    return;
  }

  const addedComments = findAddedComments(context.baseRef, context.headSha);
  if (addedComments.length === 0) {
    console.log('No new code comments introduced by this PR.');
    return;
  }

  await postMissingNudges(context, addedComments);
}

async function postMissingNudges(
  context: PullRequestContext,
  addedComments: AddedComment[],
): Promise<void> {
  const alreadyNudged = collectNudgedLines(await fetchExistingComments(context));

  let posted = 0;
  for (const comment of addedComments) {
    if (alreadyNudged.has(markerKey(comment))) continue;
    await createReviewComment(context, comment, nudgeBody(comment));
    posted += 1;
  }

  console.log(
    `Posted ${posted} comment nudge(s); skipped ${addedComments.length - posted} already present.`,
  );
}

function collectNudgedLines(existing: ExistingReviewComment[]): Set<string> {
  const keys = new Set<string>();
  for (const comment of existing) {
    const marker = extractMarkerKey(comment.body);
    if (marker) keys.add(marker);
  }
  return keys;
}

function nudgeBody(comment: AddedComment): string {
  return `${hiddenMarker(comment)}\n${COMMENT_NUDGE_BODY}`;
}

function hiddenMarker(comment: AddedComment): string {
  return `${COMMENT_NUDGE_MARKER_PREFIX}${markerKey(comment)} -->`;
}

function markerKey(comment: AddedComment): string {
  return `${comment.file}:${comment.startLine}-${comment.endLine}`;
}

function extractMarkerKey(body: string): string | null {
  const start = body.indexOf(COMMENT_NUDGE_MARKER_PREFIX);
  if (start === -1) return null;
  const from = start + COMMENT_NUDGE_MARKER_PREFIX.length;
  const end = body.indexOf(' -->', from);
  if (end === -1) return null;
  return body.slice(from, end);
}

main().catch(error => {
  console.error(`Error: ${error instanceof Error ? error.message : String(error)}`);
  process.exit(1);
});
