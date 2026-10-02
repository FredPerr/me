import type { RemoteProjectLinkData } from "@/domain/work-tracking/RemoteProjectLink";
import { RemoteProjectLinks } from "@/domain/work-tracking/RemoteProjectLinks";
import { type BranchNamingData, BranchNamingPolicy } from "@/models/BranchNaming";
import {
	type ContextStatus,
	DEFAULT_CONTEXT_STATUS,
	IDLE_CONTEXT_STATUS,
	isContextStatus,
} from "@/models/ContextStatus";
import { normalizeFolderPath, resolvePath } from "@/utils/resolvePath";

export class Repository {
	constructor(
		public readonly id: string,
		public readonly name: string,
		public readonly relPath: string,
		public readonly postCheckoutCommand?: string,
	) {}

	async resolveAbsolutePath(projectPath: string): Promise<string> {
		return resolvePath(this.relPath, { basePath: projectPath });
	}

	static create(name: string, relPath: string): Repository {
		return new Repository(crypto.randomUUID(), name, relPath);
	}

	static fromJSON(data: RepositoryData): Repository {
		return new Repository(data.id, data.name, data.relPath, data.postCheckoutCommand);
	}

	toJSON(): RepositoryData {
		return {
			id: this.id,
			name: this.name,
			relPath: this.relPath,
			postCheckoutCommand: this.postCheckoutCommand,
		};
	}
}

export class ContextBranch {
	constructor(
		public readonly repositoryId: string,
		public readonly branch: string,
		public readonly linked: boolean = false,
	) {}

	static fromJSON(data: ContextBranchData): ContextBranch {
		return new ContextBranch(data.repositoryId, data.branch, data.linked ?? false);
	}

	toJSON(): ContextBranchData {
		return {
			repositoryId: this.repositoryId,
			branch: this.branch,
			linked: this.linked,
		};
	}
}

export class Context {
	constructor(
		public readonly id: string,
		public readonly name: string,
		public readonly branches: ContextBranch[],
		public readonly isDefault: boolean = false,
		public readonly baseContextName?: string,
		public readonly pullRequestDrafts: Record<string, PullRequestDraft> = {},
		public readonly status: ContextStatus = isDefault
			? IDLE_CONTEXT_STATUS
			: DEFAULT_CONTEXT_STATUS,
		public readonly preprompt?: string,
		public readonly expanded: boolean = false,
	) {}

	getBranchForRepository(repositoryId: string): string | undefined {
		return this.branches.find((b) => b.repositoryId === repositoryId)?.branch;
	}

	getPullRequestDraft(repositoryId: string): PullRequestDraft | undefined {
		return this.pullRequestDrafts[repositoryId];
	}

	withPullRequestDrafts(drafts: Record<string, PullRequestDraft>): Context {
		return new Context(
			this.id,
			this.name,
			this.branches,
			this.isDefault,
			this.baseContextName,
			drafts,
			this.status,
			this.preprompt,
			this.expanded,
		);
	}

	withStatus(status: ContextStatus): Context {
		return new Context(
			this.id,
			this.name,
			this.branches,
			this.isDefault,
			this.baseContextName,
			this.pullRequestDrafts,
			status,
			this.preprompt,
			this.expanded,
		);
	}

	withExpanded(expanded: boolean): Context {
		return new Context(
			this.id,
			this.name,
			this.branches,
			this.isDefault,
			this.baseContextName,
			this.pullRequestDrafts,
			this.status,
			this.preprompt,
			expanded,
		);
	}

	getWorktreePath(projectPath: string, repository: Repository): string {
		const normalizedProject = normalizeFolderPath(projectPath);
		if (this.isDefault) {
			const normalizedRel = repository.relPath.startsWith("./")
				? repository.relPath.slice(2)
				: repository.relPath;
			if (normalizedRel === "." || normalizedRel === "") return normalizedProject;
			return `${normalizedProject}/${normalizedRel}`;
		}
		return `${normalizedProject}/.worktrees/${this.name}/${repository.name}`;
	}

	getContextFolderPath(projectPath: string): string {
		return `${normalizeFolderPath(projectPath)}/.worktrees/${this.name}`;
	}

	static create(
		name: string,
		repositories: Repository[],
		branchName: string,
		isDefault = false,
	): Context {
		return new Context(
			crypto.randomUUID(),
			name,
			repositories.map((repo) => new ContextBranch(repo.id, branchName)),
			isDefault,
		);
	}

	static fromJSON(data: ContextData): Context {
		const isDefault = data.isDefault ?? false;
		const status = isContextStatus(data.status)
			? data.status
			: isDefault
				? IDLE_CONTEXT_STATUS
				: DEFAULT_CONTEXT_STATUS;
		return new Context(
			data.id,
			data.name,
			(data.branches ?? []).map(ContextBranch.fromJSON),
			isDefault,
			data.baseContextName,
			data.pullRequestDrafts ?? {},
			status,
			data.preprompt,
			data.expanded ?? false,
		);
	}

	toJSON(): ContextData {
		return {
			id: this.id,
			name: this.name,
			branches: this.branches.map((b) => b.toJSON()),
			isDefault: this.isDefault,
			baseContextName: this.baseContextName,
			pullRequestDrafts: this.pullRequestDrafts,
			status: this.status,
			preprompt: this.preprompt,
			expanded: this.expanded,
		};
	}
}

export class Project {
	constructor(
		public readonly name: string,
		public readonly tag: string,
		public readonly path: string,
		public readonly repositories: Repository[],
		public readonly contexts: Context[],
		public readonly icon?: string,
		public readonly symlinks: string[] = [],
		public readonly remoteProjectLinks: RemoteProjectLinks = RemoteProjectLinks.empty(),
		public readonly branchNaming: BranchNamingPolicy = BranchNamingPolicy.default(),
	) {}

	findRepository(repositoryId: string): Repository | undefined {
		return this.repositories.find((r) => r.id === repositoryId);
	}

	effectiveRepositories(): Repository[] {
		if (this.repositories.length > 0) return this.repositories;
		return [new Repository(this.tag, this.name, ".")];
	}

	findEffectiveRepository(repositoryId: string): Repository | undefined {
		return this.effectiveRepositories().find((r) => r.id === repositoryId);
	}

	resolveContextIdePathCandidates(context: Context): string[] {
		const contextRepositories = context.branches
			.map((branch) => this.findEffectiveRepository(branch.repositoryId))
			.filter((repository): repository is Repository => repository !== undefined);
		const candidatePaths: string[] = [];
		if (contextRepositories.length === 1) {
			candidatePaths.push(context.getWorktreePath(this.path, contextRepositories[0]));
		}
		candidatePaths.push(context.getContextFolderPath(this.path), normalizeFolderPath(this.path));
		return [...new Set(candidatePaths)];
	}

	async resolveRootIdePathCandidates(): Promise<string[]> {
		const projectFolderPath = normalizeFolderPath(this.path);
		const repositories = this.effectiveRepositories();
		if (repositories.length !== 1) return [projectFolderPath];
		const repositoryPath = normalizeFolderPath(
			await repositories[0].resolveAbsolutePath(this.path),
		);
		return [...new Set([repositoryPath, projectFolderPath])];
	}

	get defaultContext(): Context | undefined {
		return this.contexts.find((c) => c.isDefault);
	}

	addContext(context: Context): Project {
		return new Project(
			this.name,
			this.tag,
			this.path,
			this.repositories,
			[...this.contexts, context],
			this.icon,
			this.symlinks,
			this.remoteProjectLinks,
			this.branchNaming,
		);
	}

	removeContext(contextId: string): Project {
		return new Project(
			this.name,
			this.tag,
			this.path,
			this.repositories,
			this.contexts.filter((c) => c.id !== contextId),
			this.icon,
			this.symlinks,
			this.remoteProjectLinks,
			this.branchNaming,
		);
	}

	withRemoteProjectLinks(remoteProjectLinks: RemoteProjectLinks): Project {
		return new Project(
			this.name,
			this.tag,
			this.path,
			this.repositories,
			this.contexts,
			this.icon,
			this.symlinks,
			remoteProjectLinks,
			this.branchNaming,
		);
	}

	static fromJSON(data: ProjectData): Project {
		return new Project(
			data.name,
			data.tag,
			data.path,
			(data.repositories ?? []).map(Repository.fromJSON),
			(data.contexts ?? []).map(Context.fromJSON),
			data.icon,
			data.symlinks ?? [],
			RemoteProjectLinks.fromJSON(data.remoteProjectLinks),
			BranchNamingPolicy.fromJSON(data.branchNaming),
		);
	}

	toJSON(): ProjectData {
		return {
			name: this.name,
			tag: this.tag,
			path: this.path,
			icon: this.icon,
			repositories: this.repositories.map((r) => r.toJSON()),
			contexts: this.contexts.map((c) => c.toJSON()),
			symlinks: this.symlinks,
			remoteProjectLinks: this.remoteProjectLinks.toJSON(),
			branchNaming: this.branchNaming.toJSON(),
		};
	}
}

export type RepositoryData = {
	id: string;
	name: string;
	relPath: string;
	postCheckoutCommand?: string;
};

export type ContextBranchData = {
	repositoryId: string;
	branch: string;
	linked?: boolean;
};

export type PullRequestDraft = {
	title: string;
	description: string;
};

export type ContextData = {
	id: string;
	name: string;
	branches: ContextBranchData[];
	isDefault?: boolean;
	baseContextName?: string;
	pullRequestDrafts?: Record<string, PullRequestDraft>;
	status?: ContextStatus;
	preprompt?: string;
	expanded?: boolean;
};

export type ProjectData = {
	name: string;
	tag: string;
	path: string;
	icon?: string;
	repositories: RepositoryData[];
	contexts: ContextData[];
	symlinks?: string[];
	remoteProjectLinks?: RemoteProjectLinkData[];
	branchNaming?: BranchNamingData;
};
