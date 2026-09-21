import fs from 'node:fs';
import ts from 'typescript';
import { readAddedLinesByFile } from './diff-lines.ts';
import type { AddedUnionType } from './types.ts';

const MIN_UNION_MEMBERS = 2;

/**
 * Returns string-literal union type aliases introduced by the PR (e.g.
 * `type Status = "active" | "archived"`), mapped to their line range. Only
 * aliases whose declaration line was added in the diff are reported, so
 * pre-existing unions are never flagged.
 */
export function findAddedUnionTypes(baseRef: string, headSha: string): AddedUnionType[] {
  const addedLinesByFile = readAddedLinesByFile(baseRef, headSha);
  const found: AddedUnionType[] = [];

  for (const [file, addedLines] of addedLinesByFile) {
    if (!fs.existsSync(file)) continue;
    found.push(...findInFile(file, addedLines));
  }

  return found;
}

function findInFile(file: string, addedLines: Set<number>): AddedUnionType[] {
  const source = ts.createSourceFile(
    file,
    fs.readFileSync(file, 'utf8'),
    ts.ScriptTarget.Latest,
    true,
  );

  const results: AddedUnionType[] = [];
  const visit = (node: ts.Node): void => {
    const alias = asStringLiteralUnionAlias(node);
    if (alias) {
      const range = lineRange(source, node);
      if (addedLines.has(range.startLine)) {
        results.push({ file, name: alias, ...range });
      }
    }
    ts.forEachChild(node, visit);
  };
  visit(source);

  return results;
}

function asStringLiteralUnionAlias(node: ts.Node): string | null {
  if (!ts.isTypeAliasDeclaration(node)) return null;
  if (!ts.isUnionTypeNode(node.type)) return null;

  const members = node.type.types;
  if (members.length < MIN_UNION_MEMBERS) return null;
  if (!members.every(isStringLiteralType)) return null;

  return node.name.text;
}

function isStringLiteralType(node: ts.TypeNode): boolean {
  return ts.isLiteralTypeNode(node) && ts.isStringLiteral(node.literal);
}

function lineRange(
  source: ts.SourceFile,
  node: ts.Node,
): { startLine: number; endLine: number } {
  // TypeScript line numbers are 0-based; GitHub expects 1-based.
  const start = source.getLineAndCharacterOfPosition(node.getStart(source)).line + 1;
  const end = source.getLineAndCharacterOfPosition(node.getEnd()).line + 1;
  return { startLine: start, endLine: end };
}
