import { TabsTab } from "@mantine/core";

type ProjectTabProps = {
	displayName: string;
	projectTag: string;
};

export function ProjectTab({ displayName, projectTag }: ProjectTabProps) {
	return <TabsTab value={projectTag} fw={700}>{displayName}</TabsTab>;
}
