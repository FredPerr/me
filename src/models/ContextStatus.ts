export const CONTEXT_STATUSES = [
	"idle",
	"ready",
	"inProgress",
	"needsHuman",
	"review",
	"done",
] as const;

export type ContextStatus = (typeof CONTEXT_STATUSES)[number];

export const DEFAULT_CONTEXT_STATUS: ContextStatus = "ready";
export const IDLE_CONTEXT_STATUS: ContextStatus = "idle";

export const CONTEXT_STATUS_COLORS: Record<ContextStatus, string> = {
	idle: "gray",
	ready: "green",
	inProgress: "grape",
	needsHuman: "orange",
	review: "cyan",
	done: "blue",
};

export function isContextStatus(value: unknown): value is ContextStatus {
	return typeof value === "string" && (CONTEXT_STATUSES as readonly string[]).includes(value);
}
