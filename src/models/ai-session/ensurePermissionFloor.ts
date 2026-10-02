import { join } from "@tauri-apps/api/path";
import { exists, mkdir, writeTextFile } from "@tauri-apps/plugin-fs";

/**
 * Writes a workspace-scoped Kiro permission policy that keeps the agent on a
 * short leash even when it runs with `--trust-all-tools`.
 *
 * Kiro resolves permissions with `deny > ask > allow` and restrictive rules
 * cannot be overridden, so a `deny` floor here blocks dangerous commands and
 * secret-file access regardless of the trust flag. The file is written once per
 * workspace and never overwritten, so a user's own edits are preserved.
 */

const PERMISSIONS_RELATIVE_DIR = ".kiro/settings";
const PERMISSIONS_FILE = "permissions.yaml";

/**
 * The seeded policy. A tight deny floor for destructive shell commands and
 * sensitive paths; everything else is left to the trust flag. Kept deliberately
 * small and readable so users can extend it.
 */
const DEFAULT_PERMISSIONS_YAML = `# Managed by the app: safety floor for trusted (--trust-all-tools) Kiro runs.
# Kiro resolves permissions as deny > ask > allow, and deny always wins, so
# these rules block dangerous actions even when all tools are trusted.
# Edit freely — this file is only created when absent, never overwritten.
rules:
  - capability: shell
    effect: deny
    match:
      - "rm -rf /*"
      - "rm -rf ~*"
      - "rm -rf .*"
      - "sudo *"
      - "chmod 777*"
      - "chmod -R 777*"
      - "mkfs*"
      - "dd * of=/dev/*"
      - ":(){ :|:& };:"
      - "shutdown*"
      - "reboot*"
      - "git push --force*"
      - "git push -f*"
      - "git push * --force*"
      - "* --no-verify"
  - capability: filesystem
    effect: deny
    match:
      - "**/.env"
      - "**/.env.*"
      - "**/*.pem"
      - "**/*.key"
      - "**/id_rsa*"
      - "**/id_ed25519*"
      - "**/.ssh/**"
      - "**/.aws/credentials"
      - "**/.npmrc"
      - "**/.netrc"
`;

/**
 * Ensures `<workingDirectory>/.kiro/settings/permissions.yaml` exists, creating
 * the directory and seeding the default policy if it does not. Returns true
 * when a file was written, false when one already existed. Failures to write
 * are swallowed (returns false) so they never block a run — the caller may log.
 */
export async function ensurePermissionFloor(workingDirectory: string): Promise<boolean> {
	try {
		const settingsDir = await join(workingDirectory, PERMISSIONS_RELATIVE_DIR);
		const filePath = await join(settingsDir, PERMISSIONS_FILE);

		if (await exists(filePath)) return false;

		if (!(await exists(settingsDir))) {
			await mkdir(settingsDir, { recursive: true });
		}
		await writeTextFile(filePath, DEFAULT_PERMISSIONS_YAML);
		return true;
	} catch {
		return false;
	}
}

export { DEFAULT_PERMISSIONS_YAML };
