import { Button, Group, Stack, TextInput } from "@mantine/core";
import { useState } from "react";
import type { Project } from "@/models/Project";

type ProjectFormProps = {
	initialProject?: Project;
	onSubmit: (project: Project) => void;
	onCancel: () => void;
};

export function ProjectForm({ initialProject, onSubmit, onCancel }: ProjectFormProps) {
	const [name, setName] = useState(initialProject?.name ?? "");
	const [tag, setTag] = useState(initialProject?.tag ?? "");
	const [path, setPath] = useState(initialProject?.path ?? "");

	const isEditing = !!initialProject;

	function handleSubmit(event: React.FormEvent) {
		event.preventDefault();
		onSubmit({
			name,
			tag,
			path,
			subprojects: initialProject?.subprojects ?? [],
		});
	}

	return (
		<form onSubmit={handleSubmit}>
			<Stack gap="sm">
				<TextInput
					label="Name"
					placeholder="My Project"
					value={name}
					onChange={(event) => setName(event.currentTarget.value)}
					required
				/>
				<TextInput
					label="Tag"
					placeholder="my-project"
					value={tag}
					onChange={(event) => setTag(event.currentTarget.value)}
					disabled={isEditing}
					required
				/>
				<TextInput
					label="Path"
					placeholder="/Users/fred/Projects/my-project"
					value={path}
					onChange={(event) => setPath(event.currentTarget.value)}
					required
				/>
				<Group justify="flex-end">
					<Button variant="subtle" onClick={onCancel}>
						Cancel
					</Button>
					<Button type="submit">{isEditing ? "Save" : "Add"}</Button>
				</Group>
			</Stack>
		</form>
	);
}
