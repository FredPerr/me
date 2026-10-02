import { join } from "@tauri-apps/api/path";
import { exists, readDir, readTextFile } from "@tauri-apps/plugin-fs";

/**
 * Single-file template locations GitHub recognizes, relative to the repository
 * root. GitHub resolves `PULL_REQUEST_TEMPLATE.md` in the repo root, in
 * `.github/`, or in `docs/`, and the name is case-insensitive on its platform.
 *
 * Docs: https://docs.github.com/communities/using-templates-to-encourage-useful-issues-and-pull-requests/creating-a-pull-request-template-for-your-repository
 */
const TEMPLATE_FILE_CANDIDATES = [
	".github/PULL_REQUEST_TEMPLATE.md",
	".github/pull_request_template.md",
	"PULL_REQUEST_TEMPLATE.md",
	"pull_request_template.md",
	"docs/PULL_REQUEST_TEMPLATE.md",
	"docs/pull_request_template.md",
];

/**
 * Directories that may hold multiple named templates
 * (`.github/PULL_REQUEST_TEMPLATE/*.md`). When several exist we cannot know
 * which the author wants, so the first one (alphabetically) is used as a
 * reasonable default — the author can still edit the generated description.
 */
const TEMPLATE_DIR_CANDIDATES = [
	".github/PULL_REQUEST_TEMPLATE",
	"PULL_REQUEST_TEMPLATE",
	"docs/PULL_REQUEST_TEMPLATE",
];

/**
 * Read the pull request template for the repository checked out at
 * `worktreePath`, or `undefined` when the repository defines none. Checks the
 * single-file locations first, then the multi-template directories.
 */
export async function readPullRequestTemplate(worktreePath: string): Promise<string | undefined> {
	for (const candidate of TEMPLATE_FILE_CANDIDATES) {
		const path = await join(worktreePath, candidate);
		if (await exists(path)) {
			return readTextFile(path);
		}
	}

	for (const candidate of TEMPLATE_DIR_CANDIDATES) {
		const dirPath = await join(worktreePath, candidate);
		if (!(await exists(dirPath))) continue;

		const entries = await readDir(dirPath);
		const markdownFiles = entries
			.filter((entry) => entry.isFile && entry.name.toLowerCase().endsWith(".md"))
			.map((entry) => entry.name)
			.sort();

		const firstTemplate = markdownFiles[0];
		if (firstTemplate) {
			return readTextFile(await join(dirPath, firstTemplate));
		}
	}

	return undefined;
}
