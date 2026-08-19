export type SubProject = {
	id: string;
	name: string;
	relPath: string;
	icon?: string;
};

export type Project = {
	name: string;
	tag: string;
	path: string;
	icon?: string;
	subprojects: SubProject[];
};
