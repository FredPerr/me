import { Button, Group, Modal, Stack, Title } from "@mantine/core";
import { useDisclosure } from "@mantine/hooks";
import { PlusIcon } from "@phosphor-icons/react";
import { useState } from "react";
import { useProjects } from "@/hooks/useProjects";
import type { Project } from "@/models/Project";
import { ProjectDirectory } from "@/models/ProjectDirectory";
import { ProjectForm } from "./ProjectForm";
import { ProjectList } from "./ProjectList";

export function ProjectSettings() {
	const { projects, reload } = useProjects();
	const [editingProject, setEditingProject] = useState<Project | undefined>();
	const [opened, { open, close }] = useDisclosure(false);

	function handleAdd() {
		setEditingProject(undefined);
		open();
	}

	function handleEdit(project: Project) {
		setEditingProject(project);
		open();
	}

	async function handleDelete(project: Project) {
		await ProjectDirectory.deleteProject(project.tag);
		await reload();
	}

	async function handleSubmit(project: Project) {
		await ProjectDirectory.saveProject(project);
		close();
		await reload();
	}

	return (
		<Stack gap="lg">
			<Group justify="space-between">
				<Title order={3}>Projects</Title>
				<Button leftSection={<PlusIcon />} onClick={handleAdd}>
					Add project
				</Button>
			</Group>

			<ProjectList projects={projects} onEdit={handleEdit} onDelete={handleDelete} />

			<Modal
				opened={opened}
				onClose={close}
				title={editingProject ? "Edit project" : "Add project"}
			>
				<ProjectForm initialProject={editingProject} onSubmit={handleSubmit} onCancel={close} />
			</Modal>
		</Stack>
	);
}
