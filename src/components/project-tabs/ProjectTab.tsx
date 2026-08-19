import { TabsTab } from "@mantine/core";
import { getIconComponent } from "@/components/shared/IconPicker";

type ProjectTabProps = {
	displayName: string;
	projectTag: string;
	icon?: string;
};

export function ProjectTab({ displayName, projectTag, icon }: ProjectTabProps) {
	const Icon = icon ? getIconComponent(icon) : null;

	return (
		<TabsTab value={projectTag} fw={700} leftSection={Icon ? <Icon size={16} /> : undefined}>
			{displayName}
		</TabsTab>
	);
}
