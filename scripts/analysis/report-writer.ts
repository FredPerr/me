import fs from 'node:fs';
import { commentPath } from './config.ts';

export function publishComment(body: string): void {
  fs.writeFileSync(commentPath(), body);
  appendToStepSummary(body);
}

function appendToStepSummary(body: string): void {
  const summaryPath = process.env.GITHUB_STEP_SUMMARY;
  if (summaryPath) {
    fs.appendFileSync(summaryPath, body);
    return;
  }
  console.log(body);
}
