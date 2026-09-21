import { execFileSync } from 'node:child_process';
import type { AddedComment } from './types.ts';

const DIFF_FILE_HEADER = /^\+\+\+ b\/(.+)$/;
const DIFF_HUNK_HEADER = /^@@ -\d+(?:,\d+)? \+(\d+)(?:,\d+)? @@/;
const CODE_FILE_PATTERN = /^src\/.+\.(ts|tsx|mts|cts)$/;

const DIRECTIVE_PATTERNS = [
  'biome-ignore',
  '@ts-',
];

/**
 * Returns the code comments introduced by the PR, mapped to their new-file line
 * numbers. Only additions in src/ TypeScript files are considered, so existing
 * comments never get flagged.
 */
export function findAddedComments(baseRef: string, headSha: string): AddedComment[] {
  const diff = readDiff(baseRef, headSha);
  return collectAddedComments(diff);
}

function readDiff(baseRef: string, headSha: string): string {
  const mergeBase = execFileSync('git', ['merge-base', `origin/${baseRef}`, headSha], {
    encoding: 'utf8',
  }).trim();
  return execFileSync('git', ['diff', '--unified=0', mergeBase, headSha], {
    encoding: 'utf8',
    maxBuffer: 64 * 1024 * 1024,
  });
}

function collectAddedComments(diff: string): AddedComment[] {
  const blocks: AddedComment[] = [];
  let currentFile: string | null = null;
  let newLineNumber = 0;
  let openBlock: AddedComment | null = null;

  const closeBlock = (): void => {
    if (openBlock) {
      blocks.push(openBlock);
      openBlock = null;
    }
  };

  for (const rawLine of diff.split('\n')) {
    const fileHeader = rawLine.match(DIFF_FILE_HEADER);
    if (fileHeader) {
      closeBlock();
      currentFile = fileHeader[1];
      continue;
    }

    const hunkHeader = rawLine.match(DIFF_HUNK_HEADER);
    if (hunkHeader) {
      closeBlock();
      newLineNumber = Number(hunkHeader[1]);
      continue;
    }

    if (!isAddedContentLine(rawLine)) continue;

    const content = rawLine.slice(1);
    const isComment =
      currentFile !== null && isTrackedCodeFile(currentFile) && isCodeComment(content);

    if (isComment && currentFile) {
      const isContiguous = openBlock !== null && openBlock.endLine === newLineNumber - 1;
      if (isContiguous && openBlock) {
        openBlock.endLine = newLineNumber;
      } else {
        closeBlock();
        openBlock = { file: currentFile, startLine: newLineNumber, endLine: newLineNumber };
      }
    } else {
      closeBlock();
    }
    newLineNumber += 1;
  }

  closeBlock();
  return blocks;
}

function isAddedContentLine(rawLine: string): boolean {
  return rawLine.startsWith('+') && !rawLine.startsWith('+++');
}

function isTrackedCodeFile(file: string): boolean {
  return CODE_FILE_PATTERN.test(file);
}

/**
 * Detects a line-leading `//` or `/* *​/` comment. JSDoc (`/**`) is intentional
 * documentation and is skipped, as are tooling directives. This is deliberately
 * conservative: comments trailing code on the same line are not flagged, which
 * avoids false positives from `//` inside string literals.
 */
export function isCodeComment(content: string): boolean {
  const trimmed = content.trim();

  if (trimmed.startsWith('/**')) return false;
  if (isDirective(trimmed)) return false;

  const isLineComment = trimmed.startsWith('//');
  const isBlockComment = trimmed.startsWith('/*') || trimmed.startsWith('*');
  return isLineComment || isBlockComment;
}

function isDirective(trimmed: string): boolean {
  const withoutMarker = trimmed.replace(/^(\/\/|\/\*|\*)\s*/, '');
  return DIRECTIVE_PATTERNS.some(directive => withoutMarker.startsWith(directive));
}
