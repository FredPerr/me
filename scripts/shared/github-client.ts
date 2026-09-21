import type { PullRequestContext } from './types.ts';

const GITHUB_API = 'https://api.github.com';

export const PER_PAGE = 100;

export async function request(
  context: PullRequestContext,
  method: string,
  path: string,
  payload?: unknown,
): Promise<unknown> {
  const response = await fetch(`${GITHUB_API}${path}`, {
    method,
    headers: {
      authorization: `Bearer ${context.token}`,
      accept: 'application/vnd.github+json',
      'x-github-api-version': '2022-11-28',
      'content-type': 'application/json',
    },
    body: payload ? JSON.stringify(payload) : undefined,
  });

  if (!response.ok) {
    const detail = await response.text();
    throw new Error(`GitHub API ${method} ${path} failed (${response.status}): ${detail}`);
  }
  return response.json();
}
