import { load } from "@tauri-apps/plugin-store";
import type { GitProviderId } from "./GitProvider";

export type GitAccount = {
	provider: GitProviderId;
	login: string;
	name: string | null;
	avatarUrl: string | null;
};

const ACCOUNTS_STORE_FILE = "git-accounts.json";
const ACCOUNTS_KEY = "accounts";

async function openStore() {
	return load(ACCOUNTS_STORE_FILE, { autoSave: true });
}

export const GitAccountRepository = {
	async loadAll(): Promise<GitAccount[]> {
		const store = await openStore();
		const stored = await store.get<GitAccount[]>(ACCOUNTS_KEY);
		return stored ?? [];
	},

	async find(provider: GitProviderId): Promise<GitAccount | null> {
		const accounts = await GitAccountRepository.loadAll();
		return accounts.find((account) => account.provider === provider) ?? null;
	},

	async save(account: GitAccount): Promise<void> {
		const store = await openStore();
		const accounts = await GitAccountRepository.loadAll();
		const others = accounts.filter((existing) => existing.provider !== account.provider);
		await store.set(ACCOUNTS_KEY, [...others, account]);
		await store.save();
	},

	async remove(provider: GitProviderId): Promise<void> {
		const store = await openStore();
		const accounts = await GitAccountRepository.loadAll();
		const remaining = accounts.filter((account) => account.provider !== provider);
		await store.set(ACCOUNTS_KEY, remaining);
		await store.save();
	},
};
