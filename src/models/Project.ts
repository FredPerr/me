export type SubProject = {
	name: string;
	path: string;
};

export type Project = {
	name: string;
	tag: string;
	path: string;
	subprojects: SubProject[];
};
