import type { CliArgs } from './types.ts';

const LOCAL_FLAG = '--local';
const CHANGED_FLAG = '--changed';

export function parseCliArgs(argv: string[]): CliArgs {
  return {
    isLocal: argv.includes(LOCAL_FLAG),
    changedFiles: parseChangedFiles(argv),
  };
}

function parseChangedFiles(argv: string[]): string[] {
  const changedIndex = argv.indexOf(CHANGED_FLAG);
  if (changedIndex === -1) return [];
  return argv.slice(changedIndex + 1);
}
