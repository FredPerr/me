import { execFileSync } from 'node:child_process';

const DIFF_FILE_HEADER = /^\+\+\+ b\/(.+)$/;
const DIFF_HUNK_HEADER = /^@@ -\d+(?:,\d+)? \+(\d+)(?:,\d+)? @@/;
const CODE_FILE_PATTERN = /^src\/.+\.(ts|tsx|mts|cts)$/;

/** Added new-file line numbers per changed src/ TypeScript file. */
export type AddedLinesByFile = Map<string, Set<number>>;

export function readAddedLinesByFile(baseRef: string, headSha: string): AddedLinesByFile {
  return parseAddedLines(readDiff(baseRef, headSha));
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

function parseAddedLines(diff: string): AddedLinesByFile {
  const byFile: AddedLinesByFile = new Map();
  let currentFile: string | null = null;
  let newLineNumber = 0;

  for (const rawLine of diff.split('\n')) {
    const fileHeader = rawLine.match(DIFF_FILE_HEADER);
    if (fileHeader) {
      currentFile = CODE_FILE_PATTERN.test(fileHeader[1]) ? fileHeader[1] : null;
      continue;
    }

    const hunkHeader = rawLine.match(DIFF_HUNK_HEADER);
    if (hunkHeader) {
      newLineNumber = Number(hunkHeader[1]);
      continue;
    }

    if (!isAddedContentLine(rawLine)) continue;

    if (currentFile) {
      recordLine(byFile, currentFile, newLineNumber);
    }
    newLineNumber += 1;
  }

  return byFile;
}

function isAddedContentLine(rawLine: string): boolean {
  return rawLine.startsWith('+') && !rawLine.startsWith('+++');
}

function recordLine(byFile: AddedLinesByFile, file: string, line: number): void {
  const lines = byFile.get(file) ?? new Set<number>();
  lines.add(line);
  byFile.set(file, lines);
}
