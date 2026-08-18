export type SubProject = {
	name: string;
	relPath: string;
};

export type Project = {
	name: string;
	tag: string;
	path: string;
	subprojects: SubProject[];
};
