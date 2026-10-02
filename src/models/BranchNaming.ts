import { slugify } from "@/utils/slugify";

export const DEFAULT_BRANCH_PREFIXES: readonly string[] = ["feature", "fix", "chore", "refactor"];

export type BranchNamingData = {
	prefixes: string[];
	allowNoPrefix: boolean;
};

type BranchPrefixParseResult =
	| { valid: true; prefix: string }
	| { valid: false; reason: "empty" | "invalid" };

// biome-ignore lint/suspicious/noControlCharactersInRegex: git forbids control characters in ref names
const FORBIDDEN_REF_CHARACTERS = /[\x00-\x20\x7f~^:?*[\\]/;
const FORBIDDEN_REF_SEQUENCES = ["..", "@{", "//"];

export function parseBranchPrefix(rawPrefix: string): BranchPrefixParseResult {
	const prefix = rawPrefix.trim().replace(/^\/+|\/+$/g, "");
	if (prefix === "") return { valid: false, reason: "empty" };
	const isInvalid =
		FORBIDDEN_REF_CHARACTERS.test(prefix) ||
		FORBIDDEN_REF_SEQUENCES.some((sequence) => prefix.includes(sequence)) ||
		prefix === "@" ||
		prefix.endsWith(".") ||
		prefix.split("/").some((segment) => segment.startsWith(".") || segment.endsWith(".lock"));
	if (isInvalid) return { valid: false, reason: "invalid" };
	return { valid: true, prefix };
}

export class BranchNamingPolicy {
	private constructor(
		readonly prefixes: string[],
		private readonly allowNoPrefixSetting: boolean,
	) {}

	static create(prefixes: string[], allowNoPrefix: boolean): BranchNamingPolicy {
		const validPrefixes = prefixes.flatMap((rawPrefix) => {
			const parseResult = parseBranchPrefix(rawPrefix);
			return parseResult.valid ? [parseResult.prefix] : [];
		});
		return new BranchNamingPolicy([...new Set(validPrefixes)], allowNoPrefix);
	}

	static default(): BranchNamingPolicy {
		return BranchNamingPolicy.create([...DEFAULT_BRANCH_PREFIXES], true);
	}

	static fromJSON(data: Partial<BranchNamingData> | undefined): BranchNamingPolicy {
		return BranchNamingPolicy.create(
			data?.prefixes ?? [...DEFAULT_BRANCH_PREFIXES],
			data?.allowNoPrefix ?? true,
		);
	}

	get allowsNoPrefix(): boolean {
		return this.allowNoPrefixSetting || this.prefixes.length === 0;
	}

	get defaultPrefix(): string | null {
		return this.allowsNoPrefix ? null : this.prefixes[0];
	}

	composeBranchName(taskName: string, prefix: string | null): string {
		if (prefix === null && !this.allowsNoPrefix) {
			throw new Error("A branch prefix is required by this project");
		}
		if (prefix !== null && !this.prefixes.includes(prefix)) {
			throw new Error(`Unknown branch prefix: ${prefix}`);
		}
		const slug = slugify(taskName);
		if (slug === "") return "";
		return prefix === null ? slug : `${prefix}/${slug}`;
	}

	toJSON(): BranchNamingData {
		return { prefixes: [...this.prefixes], allowNoPrefix: this.allowNoPrefixSetting };
	}
}
