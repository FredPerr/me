import { SimpleGrid, Text } from "@mantine/core";
import { useTranslation } from "react-i18next";
import type { Context, Project } from "@/models/Project";
import { ContextCard } from "./ContextCard";

type ContextsGridProps = {
	contexts: Context[];
	project: Project;
	onDelete: (contextId: string) => void;
	searchFilter?: string;
};

export function ContextsGrid({ contexts, project, onDelete, searchFilter }: ContextsGridProps) {
	const { t } = useTranslation();
	const filtered = searchFilter
		? contexts.filter((c) => c.name.toLowerCase().includes(searchFilter.toLowerCase()))
		: contexts;

	if (filtered.length === 0) {
		return <Text size="sm" c="dimmed">{t("contexts.noContexts")}</Text>;
	}

	return (
		<SimpleGrid cols={{ base: 1, sm: 2, md: 3 }} spacing="sm">
			{filtered.map((context) => (
				<ContextCard key={context.id} context={context} project={project} onDelete={onDelete} />
			))}
		</SimpleGrid>
	);
}
