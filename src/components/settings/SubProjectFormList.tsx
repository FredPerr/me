import { ActionIcon, Group, Stack, TextInput, Tooltip } from "@mantine/core";
import { PlusIcon, TrashIcon } from "@phosphor-icons/react";
import { IconPicker } from "@/components/shared/IconPicker";
import type { SubProject } from "@/models/Project";

type SubProjectFormListProps = {
	subprojects: SubProject[];
	onChange: (subprojects: SubProject[]) => void;
};

export function SubProjectFormList({ subprojects, onChange }: SubProjectFormListProps) {
	function handleAdd() {
		onChange([...subprojects, { id: crypto.randomUUID(), name: "", relPath: "" }]);
	}

	function handleRemove(index: number) {
		onChange(subprojects.filter((_, i) => i !== index));
	}

	function handleUpdate(index: number, field: keyof SubProject, value: string) {
		const updated = subprojects.map((sp, i) => (i === index ? { ...sp, [field]: value } : sp));
		onChange(updated);
	}

	return (
		<Stack gap="xs">
			<Group justify="space-between">
				<span style={{ fontSize: "var(--mantine-font-size-sm)", fontWeight: 500 }}>
					Subprojects
				</span>
				<Tooltip label="Add subproject">
					<ActionIcon variant="subtle" size="sm" onClick={handleAdd} aria-label="Add subproject">
						<PlusIcon size={16} />
					</ActionIcon>
				</Tooltip>
			</Group>
			{subprojects.map((subproject, index) => (
				<Group key={subproject.id} gap="xs" align="flex-end">
					<IconPicker
						value={subproject.icon ?? ""}
						onChange={(value) => handleUpdate(index, "icon", value)}
					/>
					<TextInput
						label="Name"
						placeholder="backend"
						required
						value={subproject.name}
						onChange={(e) => handleUpdate(index, "name", e.currentTarget.value)}
						style={{ flex: 1 }}
						size="xs"
					/>
					<TextInput
						label="Path"
						placeholder="./backend"
						required
						value={subproject.relPath}
						onChange={(e) => handleUpdate(index, "relPath", e.currentTarget.value)}
						style={{ flex: 2 }}
						size="xs"
					/>
					<ActionIcon
						variant="subtle"
						color="red"
						size="sm"
						onClick={() => handleRemove(index)}
						aria-label="Remove subproject"
					>
						<TrashIcon size={14} />
					</ActionIcon>
				</Group>
			))}
		</Stack>
	);
}
