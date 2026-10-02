import { Alert, Badge, Button, Code, Group, Stack, Text } from "@mantine/core";
import { ShieldCheckIcon } from "@phosphor-icons/react";
import { useTranslation } from "react-i18next";
import { useAgentChatActions } from "@/components/agent-chat/AgentChatContext";
import type {
	PermissionDecision,
	PermissionRequestPart as PermissionRequestPartModel,
} from "@/models/agent-chat/AgentMessage";

type PermissionRequestPartProps = {
	part: PermissionRequestPartModel;
};

const DECISION_LABEL_KEY: Record<PermissionDecision, string> = {
	allowOnce: "agentChat.permission.allowOnce",
	allowAlways: "agentChat.permission.allowAlways",
	deny: "agentChat.permission.deny",
};

/**
 * Renders a CLI permission request with allow/deny controls. Once resolved, it
 * shows the decision instead of the buttons. Decisions are relayed through the
 * chat actions context, so this component never touches the session directly.
 */
export function PermissionRequestPart({ part }: PermissionRequestPartProps) {
	const { t } = useTranslation();
	const { respondToPermission, supportsPermissions } = useAgentChatActions();

	const isResolved = part.resolution !== undefined;

	return (
		<Alert
			variant="light"
			color={isResolved ? "gray" : "yellow"}
			icon={<ShieldCheckIcon size={18} />}
			title={part.title}
		>
			<Stack gap="xs">
				{part.detail && (
					<Code block fz="xs">
						{part.detail}
					</Code>
				)}

				{isResolved ? (
					<Badge variant="light" color="gray">
						{t(DECISION_LABEL_KEY[part.resolution ?? "deny"])}
					</Badge>
				) : (
					<Group gap="xs">
						<Button
							size="xs"
							color="teal"
							disabled={!supportsPermissions}
							onClick={() => respondToPermission(part.requestId, "allowOnce")}
						>
							{t("agentChat.permission.allowOnce")}
						</Button>
						<Button
							size="xs"
							variant="light"
							color="teal"
							disabled={!supportsPermissions}
							onClick={() => respondToPermission(part.requestId, "allowAlways")}
						>
							{t("agentChat.permission.allowAlways")}
						</Button>
						<Button
							size="xs"
							variant="light"
							color="red"
							disabled={!supportsPermissions}
							onClick={() => respondToPermission(part.requestId, "deny")}
						>
							{t("agentChat.permission.deny")}
						</Button>
						{!supportsPermissions && (
							<Text size="xs" c="dimmed">
								{t("agentChat.permission.unsupported")}
							</Text>
						)}
					</Group>
				)}
			</Stack>
		</Alert>
	);
}
