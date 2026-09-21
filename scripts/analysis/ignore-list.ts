import fs from 'node:fs';

const DIRECTORY_PATTERN = /\/\*\*?$/;
const COMMENT_PREFIX = '#';

export class IgnoreList {
  private readonly patterns: string[];

  constructor(patterns: string[]) {
    this.patterns = patterns;
  }

  static fromFile(ignorePath: string): IgnoreList {
    if (!fs.existsSync(ignorePath)) return new IgnoreList([]);
    return new IgnoreList(parsePatterns(fs.readFileSync(ignorePath, 'utf8')));
  }

  isStable(file: string): boolean {
    return this.patterns.some(pattern => matchesPattern(file, pattern));
  }
}

function parsePatterns(contents: string): string[] {
  return contents
    .split('\n')
    .map(line => line.trim())
    .filter(isMeaningfulLine);
}

function isMeaningfulLine(line: string): boolean {
  return line.length > 0 && !line.startsWith(COMMENT_PREFIX);
}

function matchesPattern(file: string, pattern: string): boolean {
  const directoryPrefix = pattern.replace(DIRECTORY_PATTERN, '/');
  if (pattern !== directoryPrefix) return file.startsWith(directoryPrefix);
  return file === pattern;
}
