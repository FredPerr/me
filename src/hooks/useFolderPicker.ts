import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { commands } from "@/bindings";
import { getLastPathSegment } from "@/utils/segmentPath";
import { slugify } from "@/utils/slugify";

type FolderPickerResult = {
	path: string;
	suggestedName: string;
};

export async function pickFolder(): Promise<FolderPickerResult | null> {
	const selected = await openDialog({ directory: true, multiple: false });

	if (!selected) return null;

	const lastSegment = getLastPathSegment(selected);
	const suggestedName = slugify(lastSegment);

	return { path: selected, suggestedName };
}

type RepoFolderPickerResult = FolderPickerResult & {
	isGitRepository: boolean;
};

export async function pickRepositoryFolder(): Promise<RepoFolderPickerResult | null> {
	const result = await pickFolder();
	if (!result) return null;

	const isGitRepository = await commands.checkIsGitRepository(result.path);

	return { ...result, isGitRepository };
}
