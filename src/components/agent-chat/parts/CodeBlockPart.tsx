import { Box, CopyButton, Group, Text, Tooltip, UnstyledButton } from "@mantine/core";
import { CheckIcon, CopyIcon } from "@phosphor-icons/react";
import { useTranslation } from "react-i18next";
import type { CodeBlockPart as CodeBlockPartModel } from "@/models/agent-chat/AgentMessage";

type CodeBlockPartProps = {
	part: CodeBlockPartModel;
};

/**
 * Renders a fenced code block as monospaced, horizontally scrollable text with
 * a language label and a copy button. Syntax highlighting is intentionally
 * omitted for now to avoid a heavy highlighter dependency; the structure is
 * ready for one to drop in later.
 */
export function CodeBlockPart({ part }: CodeBlockPartProps) {
	const { t } = useTranslation();

	return (
		<Box
			style={{
				border: "1px solid var(--mantine-color-dark-4)",
				borderRadius: "var(--mantine-radius-sm)",
				overflow: "hidden",
			}}
		>
			<Group
				justify="space-between"
				px="sm"
				py={4}
				bg="dark.6"
				style={{ borderBottom: "1px solid var(--mantine-color-dark-4)" }}
			>
				<Text size="xs" c="dimmed" ff="monospace">
					{part.language ?? t("agentChat.code.plainText")}
				</Text>
				<CopyButton value={part.code}>
					{({ copied, copy }) => (
						<Tooltip label={copied ? t("agentChat.code.copied") : t("agentChat.code.copy")}>
							<UnstyledButton onClick={copy} aria-label={t("agentChat.code.copy")}>
								{copied ? (
									<CheckIcon size={14} color="var(--mantine-color-teal-4)" />
								) : (
									<CopyIcon size={14} color="var(--mantine-color-dimmed)" />
								)}
							</UnstyledButton>
						</Tooltip>
					)}
				</CopyButton>
			</Group>
			<Box
				component="pre"
				p="sm"
				m={0}
				fz="xs"
				ff="monospace"
				style={{ overflowX: "auto", whiteSpace: "pre", lineHeight: 1.5 }}
			>
				{part.code}
			</Box>
		</Box>
	);
}
