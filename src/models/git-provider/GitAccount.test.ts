import { beforeEach, describe, expect, it, vi } from "vitest";
import { type GitAccount, GitAccountRepository } from "./GitAccount";

const storeState = new Map<string, unknown>();
const setSpy = vi.fn(async (key: string, value: unknown) => {
	storeState.set(key, value);
});
const saveSpy = vi.fn(async () => {});

vi.mock("@tauri-apps/plugin-store", () => ({
	load: async () => ({
		get: async (key: string) => storeState.get(key),
		set: setSpy,
		save: saveSpy,
	}),
}));

const GITHUB_LOGIN = "octocat";
const OTHER_LOGIN = "hubot";

function buildAccount(overrides?: Partial<GitAccount>): GitAccount {
	return {
		provider: "github",
		login: GITHUB_LOGIN,
		name: "The Octocat",
		avatarUrl: null,
		...overrides,
	};
}

describe("GitAccountRepository", () => {
	beforeEach(() => {
		storeState.clear();
		vi.clearAllMocks();
	});

	it("returns an empty list when no accounts are stored", async () => {
		const accounts = await GitAccountRepository.loadAll();

		expect(accounts).toEqual([]);
	});

	it("saves a new account", async () => {
		const account = buildAccount();

		await GitAccountRepository.save(account);

		expect(await GitAccountRepository.loadAll()).toEqual([account]);
	});

	it("replaces the existing account for the same provider instead of duplicating", async () => {
		await GitAccountRepository.save(buildAccount({ login: GITHUB_LOGIN }));

		await GitAccountRepository.save(buildAccount({ login: OTHER_LOGIN }));

		const accounts = await GitAccountRepository.loadAll();
		expect(accounts).toHaveLength(1);
		expect(accounts[0].login).toBe(OTHER_LOGIN);
	});

	it("finds a stored account by provider", async () => {
		const account = buildAccount();
		await GitAccountRepository.save(account);

		const found = await GitAccountRepository.find("github");

		expect(found).toEqual(account);
	});

	it("returns null when finding a provider that is not connected", async () => {
		const found = await GitAccountRepository.find("github");

		expect(found).toBeNull();
	});

	it("removes the account for a provider", async () => {
		await GitAccountRepository.save(buildAccount());

		await GitAccountRepository.remove("github");

		expect(await GitAccountRepository.loadAll()).toEqual([]);
	});
});
