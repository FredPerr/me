import { COMMENT_MARKER } from './config.ts';
import type { Changelog, ChangelogEntry } from './types.ts';

export function renderChangelogComment(changelog: Changelog): string {
  const lines: string[] = [
    COMMENT_MARKER,
    '## Changelog',
    '',
    `Target branch: \`${changelog.targetBranch}\``,
    '',
  ];

  if (changelog.sections.length === 0) {
    lines.push('_No changelog-worthy commits found._', '');
  } else {
    for (const section of changelog.sections) {
      lines.push(`### ${section.title}`);
      lines.push(...section.entries.map(renderEntry));
      lines.push('');
    }
  }

  lines.push(
    '---',
    '<sub>This changelog is generated automatically and updates on every push.</sub>',
  );

  return lines.join('\n');
}

function renderEntry(entry: ChangelogEntry): string {
  const scope = entry.scope ? ` **${entry.scope}**` : '';
  const breaking = entry.isBreaking ? ' :boom:' : '';
  return `- ${entry.description}${scope}${breaking} (${entry.shortSha})`;
}
