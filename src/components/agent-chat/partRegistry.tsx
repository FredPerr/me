import type { ComponentType } from "react";
import { CodeBlockPart } from "@/components/agent-chat/parts/CodeBlockPart";
import { ErrorPart } from "@/components/agent-chat/parts/ErrorPart";
import { MarkdownPart } from "@/components/agent-chat/parts/MarkdownPart";
import { PermissionRequestPart } from "@/components/agent-chat/parts/PermissionRequestPart";
import { StreamLinePart } from "@/components/agent-chat/parts/StreamLinePart";
import { TextPart } from "@/components/agent-chat/parts/TextPart";
import { ToolCallPart } from "@/components/agent-chat/parts/ToolCallPart";
import type { MessagePart } from "@/models/agent-chat/AgentMessage";

/** A renderer for one concrete part type: it receives that exact part. */
type PartRenderer<T extends MessagePart> = ComponentType<{ part: T }>;

/**
 * Maps each part `type` to its renderer. This is the extension point for richer
 * output: add a part type to the model and register its component here, and the
 * chat transcript renders it with no other changes. The mapped type ties each
 * renderer to its matching part variant so registrations stay type-safe.
 */
type PartRegistry = {
	[Part in MessagePart as Part["type"]]: PartRenderer<Part>;
};

const PART_REGISTRY: PartRegistry = {
	text: TextPart,
	markdown: MarkdownPart,
	code: CodeBlockPart,
	toolCall: ToolCallPart,
	streamLine: StreamLinePart,
	permissionRequest: PermissionRequestPart,
	error: ErrorPart,
};

type RenderPartProps = {
	part: MessagePart;
};

/**
 * Renders a single part by dispatching on its `type` through the registry.
 * Centralizing the lookup keeps the message component agnostic of the part set.
 */
export function RenderPart({ part }: RenderPartProps) {
	// The registry is keyed by the discriminant, so the looked-up renderer
	// always matches `part`; a small cast bridges the per-type union to the
	// component's prop without widening the public types.
	const Renderer = PART_REGISTRY[part.type] as PartRenderer<MessagePart>;
	return <Renderer part={part} />;
}
