import { Button, Group, Modal } from "@mantine/core";
import type { ProjectView } from "@/bindings";

type ProjectConfirmDeleteModalProps = {
	project: ProjectView;
	onDelete: (project: ProjectView) => void;
	onCancel: () => void;
	opened: boolean;
};

export function ProjectConfirmDeleteModal({
	project,
	onDelete,
	onCancel,
	opened,
}: ProjectConfirmDeleteModalProps) {
	const handleDelete = () => {
		onDelete(project);
	};

	return (
		<Modal opened={opened} onClose={onCancel} centered title="Confirm Project Deletion">
			<Group mt="xl" justify="space-between">
				<Button onClick={onCancel} color="red">
					Cancel
				</Button>
				<Button onClick={handleDelete} bg="red">
					Confirm Delete
				</Button>
			</Group>
		</Modal>
	);
}
