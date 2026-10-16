import type { HostKindView } from "@/bindings";

type GitHostSignInMethod = "browser" | "token";

type GitHostDisplay = {
	displayNameKey: string;
	signInMethod: GitHostSignInMethod;
};

export const GIT_HOSTS: Record<HostKindView, GitHostDisplay> = {
	github: { displayNameKey: "settings.gitHosting.github", signInMethod: "browser" },
	bitbucket: { displayNameKey: "settings.gitHosting.bitbucket", signInMethod: "token" },
};

export const GIT_HOST_KINDS: readonly HostKindView[] = ["github", "bitbucket"];
