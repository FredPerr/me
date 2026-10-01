import {
	ActionIcon,
	Button,
	Popover,
	Select,
	Stack,
	Text,
	TextInput,
	Tooltip,
} from "@mantine/core";
import { MagicWandIcon } from "@phosphor-icons/react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import type { BranchNamingPolicy } from "@/models/BranchNaming";

const NO_PREFIX_VALUE = "~none";

type BranchNameFromTaskPopoverProps = {
	branchNaming: BranchNamingPolicy;
	opened: boolean;
	onOpenedChange: (opened: boolean) => void;
	onApply: (branchName: string) => void;
};

export function BranchNameFromTaskPopover({
	branchNaming,
	opened,
	onOpenedChange,
	onApply,
}: BranchNameFromTaskPopoverProps) {
	const { t } = useTranslation();
	const defaultPrefixValue = branchNaming.defaultPrefix ?? NO_PREFIX_VALUE;
	const [taskName, setTaskName] = useState("");
	const [selectedPrefixValue, setSelectedPrefixValue] = useState(defaultPrefixValue);

	const prefixOptions = [
		...(branchNaming.allowsNoPrefix
			? [{ value: NO_PREFIX_VALUE, label: t("contexts.noBranchPrefix") }]
			: []),
		...branchNaming.prefixes.map((prefix) => ({ value: prefix, label: `${prefix}/` })),
	];
	const isSelectedPrefixAvailable = prefixOptions.some(
		(option) => option.value === selectedPrefixValue,
	);
	const effectivePrefixValue = isSelectedPrefixAvailable ? selectedPrefixValue : defaultPrefixValue;
	const generatedBranchName = branchNaming.composeBranchName(
		taskName,
		effectivePrefixValue === NO_PREFIX_VALUE ? null : effectivePrefixValue,
	);

	function open() {
		setTaskName("");
		setSelectedPrefixValue(defaultPrefixValue);
		onOpenedChange(true);
	}

	function close() {
		onOpenedChange(false);
	}

	function apply() {
		if (generatedBranchName === "") return;
		onApply(generatedBranchName);
		close();
	}

	return (
		<Popover opened={opened} onChange={onOpenedChange} position="bottom-end" width={320} trapFocus>
			<Popover.Target>
				<Tooltip label={t("contexts.branchFromTask")}>
					<ActionIcon
						variant="subtle"
						size="sm"
						onClick={opened ? close : open}
						aria-label={t("contexts.branchFromTask")}
						aria-expanded={opened}
					>
						<MagicWandIcon size={16} />
					</ActionIcon>
				</Tooltip>
			</Popover.Target>
			<Popover.Dropdown>
				<Stack gap="xs">
					<TextInput
						label={t("contexts.taskName")}
						placeholder={t("contexts.taskNamePlaceholder")}
						value={taskName}
						onChange={(event) => setTaskName(event.currentTarget.value)}
						onKeyDown={(event) => {
							if (event.key === "Enter") {
								event.preventDefault();
								apply();
							}
						}}
						data-autofocus
						size="xs"
					/>
					<Select
						label={t("contexts.branchPrefix")}
						data={prefixOptions}
						value={effectivePrefixValue}
						onChange={(value) => {
							if (value !== null) setSelectedPrefixValue(value);
						}}
						allowDeselect={false}
						comboboxProps={{ withinPortal: false }}
						size="xs"
					/>
					{generatedBranchName !== "" && (
						<Text size="xs" c="dimmed">
							{t("contexts.generatedBranchPreview", { branch: generatedBranchName })}
						</Text>
					)}
					<Button size="xs" onClick={apply} disabled={generatedBranchName === ""}>
						{t("contexts.applyBranchName")}
					</Button>
				</Stack>
			</Popover.Dropdown>
		</Popover>
	);
}
