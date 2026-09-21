import { parseCliArgs } from './cli.ts';
import { CHANGE_GRAPH_MARKER } from './config.ts';
import { buildChangeGraph } from './change-graph.ts';
import {
  buildChangeGraphComment,
  buildEmptyChangeGraphComment,
} from './change-graph-comment.ts';
import { loadDependencyGraph } from './dependency-graph.ts';
import { upsertIssueComment } from '../shared/issue-comments.ts';
import { readPullRequestContext } from '../shared/pull-request-context.ts';

async function main(): Promise<void> {
  const args = parseCliArgs(process.argv);
  if (args.changedFiles.length === 0) {
    console.log('No changed files provided; skipping change graph.');
    return;
  }

  const graph = buildChangeGraph(loadDependencyGraph(), args.changedFiles);
  const body = graph.edges.length === 0
    ? buildEmptyChangeGraphComment()
    : buildChangeGraphComment(graph);

  await publish(body);
}

async function publish(body: string): Promise<void> {
  const context = readPullRequestContext();
  if (!context) {
    console.log(body);
    return;
  }
  await upsertIssueComment(context, CHANGE_GRAPH_MARKER, body);
  console.log('Change graph comment upserted.');
}

main().catch(error => {
  console.error(`Error: ${error instanceof Error ? error.message : String(error)}`);
  process.exit(1);
});
