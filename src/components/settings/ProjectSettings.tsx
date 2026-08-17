import { Button, Group, Modal, Stack, Title } from "@mantine/core";
import { useDisclosure } from "@mantine/hooks";
import { PlusIcon } from "@phosphor-icons/react";
import { useCallback, useEffect, useState } from "react";
import type { Project } from "@/models/Project";
import { ProjectDirectory } from "@/models/ProjectDirectory";
import { ProjectForm } from "./ProjectForm";
import { ProjectList } from "./ProjectList";

export function ProjectSettings() {
	const [projects, setProjects] = useState<Project[]>([]);
	const [editingProject, setEditingProject] = useState<Project | undefined>();
	const [opened, { open, close }] = useDisclosure(false);

	const loadProjects = useCallback(async () => {
		const loaded = await ProjectDirectory.loadAllProjects();
		setProjects(loaded);
	}, []);

	useEffect(() => {
		loadProjects();
	}, [loadProjects]);

	function handleAdd() {
		setEditingProject(undefined);
		open();
	}

	function handleEdit(project: Project) {
		setEditingProject(project);
		open();
	}

	async function handleDelete(project: Project) {
		const updated = projects.filter((p) => p.tag !== project.tag);
		setProjects(updated);
	}

	async function handleSubmit(project: Project) {
		await ProjectDirectory.saveProject(project);
		close();
		await loadProjects();
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
