import type { ProviderKind as GeneratedProviderKind } from "@/bindings";

export enum ProviderKind {
	Teamwork = "teamwork",
}

export const DEFAULT_PROVIDER_KIND = ProviderKind.Teamwork;

const BY_VALUE: Record<GeneratedProviderKind, ProviderKind> = {
	teamwork: ProviderKind.Teamwork,
};

/** The matching enum for a provider kind read from the generated bindings. */
export function toProviderKind(kind: GeneratedProviderKind): ProviderKind {
	return BY_VALUE[kind];
}
