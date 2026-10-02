import { ActionIcon, Box, Group, Loader, Paper, Stack, Text } from "@mantine/core";
import { PaperPlaneTiltIcon, StopCircleIcon } from "@phosphor-icons/react";
import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import type { SlashItem, SlashProvider } from "@/models/agent-chat/slash/SlashProvider";

type ChatInputProps = {
	value: string;
	onChange: (value: string) => void;
	onSend: () => void;
	onStop: () => void;
	isBusy: boolean;
	slashProviders: SlashProvider[];
	placeholder?: string;
};

/** The active slash query: the `/token` under the cursor and its start offset. */
type SlashQuery = {
	start: number;
	text: string;
};

/**
 * Detects a slash token ending at the caret: a `/` that starts a word, followed
 * by non-whitespace. Returns its start index and the query text after the `/`,
 * or null when the caret is not in a slash token.
 */
function detectSlashQuery(value: string, caret: number): SlashQuery | null {
	const uptoCaret = value.slice(0, caret);
	const match = uptoCaret.match(/(?:^|\s)\/([^\s/]*)$/);
	if (match === null) return null;
	const start = caret - match[1].length - 1;
	return { start, text: match[1] };
}

/**
 * The chat prompt box. A textarea with a send/stop button and a slash-command
 * autocomplete: typing `/` opens a menu populated from the given providers, and
 * choosing an item inserts its reference token at the caret. Cmd/Ctrl+Enter
 * sends; while the menu is open, arrows navigate and Enter/Tab accept.
 */
export function ChatInput({
	value,
	onChange,
	onSend,
	onStop,
	isBusy,
	slashProviders,
	placeholder,
}: ChatInputProps) {
	const { t } = useTranslation();
	const textareaRef = useRef<HTMLTextAreaElement>(null);

	const [slashQuery, setSlashQuery] = useState<SlashQuery | null>(null);
	const [items, setItems] = useState<SlashItem[]>([]);
	const [loading, setLoading] = useState(false);
	const [highlighted, setHighlighted] = useState(0);

	const menuOpen = slashQuery !== null && (loading || items.length > 0);

	const refreshSlashQuery = useCallback(() => {
		const textarea = textareaRef.current;
		if (!textarea) return;
		setSlashQuery(detectSlashQuery(value, textarea.selectionStart));
	}, [value]);

	// Query the providers whenever the active slash token changes.
	useEffect(() => {
		if (slashQuery === null) {
			setItems([]);
			return;
		}
		let cancelled = false;
		setLoading(true);
		Promise.all(slashProviders.map((provider) => provider.query(slashQuery.text)))
			.then((perProvider) => {
				if (cancelled) return;
				setItems(perProvider.flat());
				setHighlighted(0);
			})
			.finally(() => {
				if (!cancelled) setLoading(false);
			});
		return () => {
			cancelled = true;
		};
	}, [slashQuery, slashProviders]);

	function applyItem(item: SlashItem) {
		if (slashQuery === null) return;
		const textarea = textareaRef.current;
		const caret = textarea
			? textarea.selectionStart
			: slashQuery.start + slashQuery.text.length + 1;
		const before = value.slice(0, slashQuery.start);
		const after = value.slice(caret);
		const insertion = `${item.insertText} `;
		onChange(`${before}${insertion}${after}`);
		setSlashQuery(null);
		setItems([]);
		// Restore focus and place the caret after the inserted token.
		requestAnimationFrame(() => {
			const node = textareaRef.current;
			if (!node) return;
			const nextCaret = before.length + insertion.length;
			node.focus();
			node.setSelectionRange(nextCaret, nextCaret);
		});
	}

	function handleKeyDown(event: React.KeyboardEvent<HTMLTextAreaElement>) {
		if (menuOpen && items.length > 0) {
			if (event.key === "ArrowDown") {
				event.preventDefault();
				setHighlighted((current) => (current + 1) % items.length);
				return;
			}
			if (event.key === "ArrowUp") {
				event.preventDefault();
				setHighlighted((current) => (current - 1 + items.length) % items.length);
				return;
			}
			if (event.key === "Enter" || event.key === "Tab") {
				event.preventDefault();
				applyItem(items[highlighted]);
				return;
			}
			if (event.key === "Escape") {
				event.preventDefault();
				setSlashQuery(null);
				return;
			}
		}

		if (event.key === "Enter" && (event.metaKey || event.ctrlKey)) {
			event.preventDefault();
			if (!isBusy && value.trim().length > 0) onSend();
		}
	}

	return (
		<Box style={{ position: "relative" }}>
			{menuOpen && (
				<Paper
					withBorder
					shadow="md"
					radius="sm"
					p={4}
					style={{
						position: "absolute",
						bottom: "calc(100% + 4px)",
						left: 0,
						right: 0,
						maxHeight: 240,
						overflowY: "auto",
						zIndex: 10,
					}}
				>
					{loading && items.length === 0 ? (
						<Group gap="xs" p="xs">
							<Loader size={14} />
							<Text size="xs" c="dimmed">
								{t("agentChat.slash.loading")}
							</Text>
						</Group>
					) : (
						<Stack gap={2}>
							{items.map((item, index) => (
								<Box
									key={item.id}
									px="xs"
									py={6}
									onMouseEnter={() => setHighlighted(index)}
									onMouseDown={(event) => {
										event.preventDefault();
										applyItem(item);
									}}
									style={{
										cursor: "pointer",
										borderRadius: "var(--mantine-radius-sm)",
										background:
											index === highlighted ? "var(--mantine-color-dark-5)" : "transparent",
									}}
								>
									<Text size="sm">{item.label}</Text>
									{item.description && (
										<Text size="xs" c="dimmed" ff="monospace">
											{item.description}
										</Text>
									)}
								</Box>
							))}
						</Stack>
					)}
				</Paper>
			)}

			<Group gap="xs" align="flex-end" wrap="nowrap">
				<textarea
					ref={textareaRef}
					value={value}
					onChange={(event) => {
						onChange(event.currentTarget.value);
						requestAnimationFrame(refreshSlashQuery);
					}}
					onKeyUp={refreshSlashQuery}
					onClick={refreshSlashQuery}
					onKeyDown={handleKeyDown}
					placeholder={placeholder ?? t("agentChat.input.placeholder")}
					disabled={isBusy}
					rows={3}
					style={{
						flex: 1,
						resize: "vertical",
						fontFamily: "inherit",
						fontSize: "var(--mantine-font-size-sm)",
						padding: "8px 10px",
						borderRadius: "var(--mantine-radius-sm)",
						border: "1px solid var(--mantine-color-dark-4)",
						background: "var(--mantine-color-dark-6)",
						color: "inherit",
					}}
				/>
				{isBusy ? (
					<ActionIcon
						size="lg"
						variant="light"
						color="red"
						onClick={onStop}
						aria-label={t("agentChat.stop")}
					>
						<StopCircleIcon size={18} />
					</ActionIcon>
				) : (
					<ActionIcon
						size="lg"
						variant="filled"
						disabled={value.trim().length === 0}
						onClick={onSend}
						aria-label={t("agentChat.send")}
					>
						<PaperPlaneTiltIcon size={16} />
					</ActionIcon>
				)}
			</Group>
		</Box>
	);
}

export { detectSlashQuery };
