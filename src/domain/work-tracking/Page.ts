export type Page<T> = {
	readonly items: readonly T[];
	readonly nextCursor?: string;
};
