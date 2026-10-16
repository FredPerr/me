import { ActionIcon, Button, Group, Input, Stack, Switch, Text, TextInput } from "@mantine/core";
import { PlusIcon, TrashIcon } from "@phosphor-icons/react";
import { useTranslation } from "react-i18next";

type BranchPrefixFormListProps = {
	prefixes: string[];
	allowNoPrefix: boolean;
	onPrefixesChange: (prefixes: string[]) => void;
	onAllowNoPrefixChange: (allowed: boolean) => void;
};

export function BranchPrefixFormList({
	prefixes,
	allowNoPrefix,
	onPrefixesChange,
	onAllowNoPrefixChange,
}: BranchPrefixFormListProps) {
	const { t } = useTranslation();

	function updatePrefix(index: number, value: string) {
		onPrefixesChange(
			prefixes.map((prefix, prefixIndex) => (prefixIndex === index ? value : prefix)),
		);
	}

	return (
		<Stack gap="xs">
			<div>
				<Input.Label>{t("settings.branchNaming.title")}</Input.Label>
				<Input.Description>{t("settings.branchNaming.description")}</Input.Description>
			</div>
			{prefixes.map((prefix, index) => (
				// biome-ignore lint/suspicious/noArrayIndexKey: prefixes have no stable ID
				<Group key={index} gap="xs" align="flex-start">
					<TextInput
						aria-label={t("settings.branchNaming.prefixLabel")}
						placeholder={t("settings.branchNaming.prefixPlaceholder")}
						value={prefix}
						onChange={(event) => updatePrefix(index, event.currentTarget.value)}
						rightSection={
							<Text size="xs" c="dimmed">
								/
							</Text>
						}
						size="xs"
						style={{ flex: 1 }}
					/>
					<ActionIcon
						variant="subtle"
						color="red"
						size="sm"
						mt={4}
						onClick={() =>
							onPrefixesChange(prefixes.filter((_, prefixIndex) => prefixIndex !== index))
						}
						aria-label={t("settings.branchNaming.removePrefix")}
					>
						<TrashIcon size={14} />
					</ActionIcon>
				</Group>
			))}
			<Group>
				<Button
					variant="outline"
					size="xs"
					color="gray"
					leftSection={<PlusIcon size={14} />}
					onClick={() => onPrefixesChange([...prefixes, ""])}
				>
					{t("settings.branchNaming.addPrefix")}
				</Button>
			</Group>
			<Switch
				label={t("settings.branchNaming.allowNoPrefix")}
				checked={allowNoPrefix}
				onChange={(event) => onAllowNoPrefixChange(event.currentTarget.checked)}
				size="xs"
			/>
		</Stack>
	);
}
