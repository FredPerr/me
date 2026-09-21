import fs from 'node:fs';
import type { PullRequestContext } from './types.ts';

export function readPullRequestContext(): PullRequestContext | null {
  const token = process.env.GITHUB_TOKEN;
  const repository = process.env.GITHUB_REPOSITORY;
  const baseRef = process.env.GITHUB_BASE_REF;
  const eventPath = process.env.GITHUB_EVENT_PATH;
  if (!token || !repository || !baseRef || !eventPath) return null;

  const [owner, repo] = repository.split('/');
  const event = readEvent(eventPath);
  const pullNumber = event?.pull_request?.number;
  const headSha = event?.pull_request?.head?.sha;
  if (!owner || !repo || !pullNumber || !headSha) return null;

  return { owner, repo, pullNumber, headSha, baseRef, token };
}

interface WebhookEvent {
  pull_request?: {
    number?: number;
    head?: { sha?: string };
  };
}

function readEvent(eventPath: string): WebhookEvent | null {
  try {
    return JSON.parse(fs.readFileSync(eventPath, 'utf8')) as WebhookEvent;
  } catch {
    return null;
  }
}
