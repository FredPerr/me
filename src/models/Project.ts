import { resolvePath } from "@/utils/resolvePath";

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
		);
	}

	getWorktreePath(projectPath: string, repository: Repository): string {
		const normalizedProject = projectPath.endsWith("/") ? projectPath.slice(0, -1) : projectPath;
		if (this.isDefault) {
			const normalizedRel = repository.relPath.startsWith("./")
				? repository.relPath.slice(2)
				: repository.relPath;
			return `${normalizedProject}/${normalizedRel}`;
		}
		return `${normalizedProject}/.worktrees/${this.name}/${repository.name}`;
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
		return new Context(
			data.id,
			data.name,
			(data.branches ?? []).map(ContextBranch.fromJSON),
			data.isDefault ?? false,
			data.baseContextName,
			data.pullRequestDrafts ?? {},
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
	) {}

	findRepository(repositoryId: string): Repository | undefined {
		return this.repositories.find((r) => r.id === repositoryId);
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
};

export type ProjectData = {
	name: string;
	tag: string;
	path: string;
	icon?: string;
	repositories: RepositoryData[];
	contexts: ContextData[];
	symlinks?: string[];
};
