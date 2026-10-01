import { ActionIcon, Button, Group, Select, Stack, Textarea, TextInput } from "@mantine/core";
import { FolderOpenIcon, PlayIcon } from "@phosphor-icons/react";
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { useAppSettings } from "@/hooks/useAppSettings";
import { pickFolder } from "@/hooks/useFolderPicker";
import type { AdapterInfo, SpawnOptions } from "@/models/ai-session/AiSession";

type SpawnSessionFormProps = {
	adapters: AdapterInfo[];
	onSpawn: (options: SpawnOptions, prompt: string) => Promise<void>;
};

/**
 * Form for starting a new AI CLI session: choose the CLI adapter, pick the
 * working directory, enter a prompt and optional agent, then spawn. The adapter
 * list is driven by the backend so new CLIs appear here without UI changes.
 */
export function SpawnSessionForm({ adapters, onSpawn }: SpawnSessionFormProps) {
	const { t } = useTranslation();
	const { settings } = useAppSettings();

	const [adapterId, setAdapterId] = useState<string | null>(adapters[0]?.id ?? null);
	const [workingDirectory, setWorkingDirectory] = useState("");
	const [prompt, setPrompt] = useState("");
	const [agent, setAgent] = useState("");
	const [spawning, setSpawning] = useState(false);
	const [error, setError] = useState<string | null>(null);

	const adapterOptions = useMemo(
		() => adapters.map((adapter) => ({ value: adapter.id, label: adapter.displayName })),
		[adapters],
	);

	const canSpawn =
		!!adapterId && workingDirectory.trim().length > 0 && prompt.trim().length > 0 && !spawning;

	async function handlePickFolder() {
		const result = await pickFolder();
		if (result) setWorkingDirectory(result.path);
	}

	async function handleSpawn() {
		if (!adapterId) return;
		setSpawning(true);
		setError(null);
		const trimmedPrompt = prompt.trim();
		const options: SpawnOptions = {
			adapterId,
			prompt: trimmedPrompt,
			workingDirectory: workingDirectory.trim(),
			agent: agent.trim() || undefined,
			shell: settings?.shell || undefined,
		};
		try {
			await onSpawn(options, trimmedPrompt);
			setPrompt("");
			setAgent("");
		} catch (spawnError) {
			setError(String(spawnError));
		} finally {
			setSpawning(false);
		}
	}

	return (
		<Stack gap="sm">
			<Group grow align="flex-end">
				<Select
					label={t("aiSessions.form.adapter")}
					placeholder={t("aiSessions.form.adapterPlaceholder")}
					data={adapterOptions}
					value={adapterId}
					onChange={setAdapterId}
					allowDeselect={false}
				/>
				<TextInput
					label={t("aiSessions.form.agent")}
					placeholder={t("aiSessions.form.agentPlaceholder")}
					value={agent}
					onChange={(event) => setAgent(event.currentTarget.value)}
				/>
			</Group>

			<TextInput
				label={t("aiSessions.form.workingDirectory")}
				placeholder={t("aiSessions.form.workingDirectoryPlaceholder")}
				value={workingDirectory}
				onChange={(event) => setWorkingDirectory(event.currentTarget.value)}
				rightSection={
					<ActionIcon
						variant="subtle"
						onClick={handlePickFolder}
						aria-label={t("aiSessions.form.pickFolder")}
					>
						<FolderOpenIcon size={16} />
					</ActionIcon>
				}
			/>

			<Textarea
				label={t("aiSessions.form.prompt")}
				placeholder={t("aiSessions.form.promptPlaceholder")}
				value={prompt}
				onChange={(event) => setPrompt(event.currentTarget.value)}
				autosize
				minRows={2}
				maxRows={6}
				error={error}
			/>

			<Group justify="flex-end">
				<Button
					leftSection={<PlayIcon size={16} />}
					disabled={!canSpawn}
					loading={spawning}
					onClick={handleSpawn}
				>
					{t("aiSessions.form.start")}
				</Button>
			</Group>
		</Stack>
	);
}
