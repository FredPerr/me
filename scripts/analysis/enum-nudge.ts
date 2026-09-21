import { ENUM_NUDGE_BODY, ENUM_NUDGE_MARKER_PREFIX } from './config.ts';
import {
  createReviewComment,
  fetchExistingComments,
  readPullRequestContext,
} from './github-review.ts';
import type { AddedUnionType, ExistingReviewComment, PullRequestContext } from './types.ts';
import { findAddedUnionTypes } from './union-types.ts';

async function main(): Promise<void> {
  const context = readPullRequestContext();
  if (!context) {
    console.log('Not running in a pull request context; skipping enum nudges.');
    return;
  }

  const addedUnionTypes = findAddedUnionTypes(context.baseRef, context.headSha);
  if (addedUnionTypes.length === 0) {
    console.log('No new string-literal union types introduced by this PR.');
    return;
  }

  await postMissingNudges(context, addedUnionTypes);
}

async function postMissingNudges(
  context: PullRequestContext,
  addedUnionTypes: AddedUnionType[],
): Promise<void> {
  const alreadyNudged = collectNudgedKeys(await fetchExistingComments(context));

  let posted = 0;
  for (const unionType of addedUnionTypes) {
    if (alreadyNudged.has(markerKey(unionType))) continue;
    await createReviewComment(context, unionType, nudgeBody(unionType));
    posted += 1;
  }

  console.log(
    `Posted ${posted} enum nudge(s); skipped ${addedUnionTypes.length - posted} already present.`,
  );
}

function collectNudgedKeys(existing: ExistingReviewComment[]): Set<string> {
  const keys = new Set<string>();
  for (const comment of existing) {
    const marker = extractMarkerKey(comment.body);
    if (marker) keys.add(marker);
  }
  return keys;
}

function nudgeBody(unionType: AddedUnionType): string {
  return `${hiddenMarker(unionType)}\n${ENUM_NUDGE_BODY}`;
}

function hiddenMarker(unionType: AddedUnionType): string {
  return `${ENUM_NUDGE_MARKER_PREFIX}${markerKey(unionType)} -->`;
}

function markerKey(unionType: AddedUnionType): string {
  return `${unionType.file}:${unionType.name}:${unionType.startLine}-${unionType.endLine}`;
}

function extractMarkerKey(body: string): string | null {
  const start = body.indexOf(ENUM_NUDGE_MARKER_PREFIX);
  if (start === -1) return null;
  const from = start + ENUM_NUDGE_MARKER_PREFIX.length;
  const end = body.indexOf(' -->', from);
  if (end === -1) return null;
  return body.slice(from, end);
}

main().catch(error => {
  console.error(`Error: ${error instanceof Error ? error.message : String(error)}`);
  process.exit(1);
});
