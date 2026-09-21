import { Button, Code, CopyButton, Group, Modal, ScrollArea, Stack, Text } from "@mantine/core";
import { CheckIcon, CopyIcon } from "@phosphor-icons/react";
import { useTranslation } from "react-i18next";

export type ErrorModalContent = {
	title: string;
	description?: string;
	details: string;
	hint?: string;
};

type ErrorModalProps = {
	opened: boolean;
	onClose: () => void;
	content: ErrorModalContent | null;
};

export function ErrorModal({ opened, onClose, content }: ErrorModalProps) {
	const { t } = useTranslation();

	if (!content) {
		return null;
	}

	return (
		<Modal opened={opened} onClose={onClose} title={content.title} size="lg" centered>
			<Stack gap="md">
				{content.description && <Text size="sm">{content.description}</Text>}
				<ScrollArea.Autosize mah={320}>
					<Code
						block
						style={{ whiteSpace: "pre-wrap", wordBreak: "break-word" }}
						aria-label={content.title}
					>
						{content.details}
					</Code>
				</ScrollArea.Autosize>
				{content.hint && (
					<Text size="sm" c="dimmed">
						{content.hint}
					</Text>
				)}
				<Group justify="flex-end">
					<CopyButton value={content.details}>
						{({ copied, copy }) => (
							<Button
								variant="subtle"
								leftSection={copied ? <CheckIcon size={16} /> : <CopyIcon size={16} />}
								onClick={copy}
							>
								{copied ? t("common.copied") : t("common.copyDetails")}
							</Button>
						)}
					</CopyButton>
					<Button onClick={onClose}>{t("common.close")}</Button>
				</Group>
			</Stack>
		</Modal>
	);
}
