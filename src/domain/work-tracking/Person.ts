import type { PersonId } from "./identifiers";

export type Person = {
	readonly id: PersonId;
	readonly displayName: string;
	readonly avatarUrl?: string;
};
