import {
	ActionIcon,
	Alert,
	Badge,
	Button,
	Checkbox,
	Code,
	Group,
	Loader,
	Modal,
	MultiSelect,
	Stack,
	Text,
	Tooltip,
} from "@mantine/core";
import { useClipboard } from "@mantine/hooks";
import {
	ArrowDownIcon,
	ArrowUpIcon,
	CheckIcon,
	CopyIcon,
	WarningIcon,
} from "@phosphor-icons/react";
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import {
	type ReviewPullRequest,
	useProjectReviewPullRequests,
} from "@/hooks/useProjectReviewPullRequests";
import type { Project } from "@/models/Project";
import { moveItem, orderReviewItems, type ReviewItem } from "@/models/review-share/ReviewStack";
import { formatSlackMessage } from "@/models/review-share/SlackMessageFormatter";

type ShareForReviewModalProps = {
	opened: boolean;
	onClose: () => void;
	project: Project;
};

function itemId(pullRequest: ReviewPullRequest): string {
	return String(pullRequest.number);
}

export function ShareForReviewModal({ opened, onClose, project }: ShareForReviewModalProps) {
	const { t } = useTranslation();
	const clipboard = useClipboard({ timeout: 1500 });
	const { connected, loading, pullRequests, error } = useProjectReviewPullRequests(project);

	// Selection state: the ordered list of included PR ids, and per-PR
	// dependency picks. Kept keyed by id so it survives reordering.
	const [orderedIds, setOrderedIds] = useState<string[]>([]);
	const [dependenciesById, setDependenciesById] = useState<Record<string, string[]>>({});

	const pullRequestById = useMemo(() => {
		const map = new Map<string, ReviewPullRequest>();
		for (const pullRequest of pullRequests) {
			map.set(itemId(pullRequest), pullRequest);
		}
		return map;
	}, [pullRequests]);

	function toggleInclude(id: string, included: boolean) {
		setOrderedIds((current) =>
			included ? [...current, id] : current.filter((existing) => existing !== id),
		);
		if (!included) {
			// Clear this PR's own dependencies and drop it from every other
			// PR's dependency list so no stale reference survives.
			setDependenciesById((current) => {
				const next: Record<string, string[]> = {};
				for (const [key, deps] of Object.entries(current)) {
					if (key === id) continue;
					next[key] = deps.filter((dependencyId) => dependencyId !== id);
				}
				return next;
			});
		}
	}

	function move(id: string, direction: -1 | 1) {
		setOrderedIds((current) => {
			const index = current.indexOf(id);
			return moveItem(current, index, index + direction);
		});
	}

	function setDependencies(id: string, dependencyIds: string[]) {
		setDependenciesById((current) => ({ ...current, [id]: dependencyIds }));
	}

	const reviewItems: ReviewItem[] = useMemo(
		() =>
			orderedIds.flatMap((id) => {
				const pullRequest = pullRequestById.get(id);
				if (!pullRequest) return [];
				return [
					{
						id,
						number: pullRequest.number,
						title: pullRequest.title,
						url: pullRequest.htmlUrl,
						dependsOn: dependenciesById[id] ?? [],
					},
				];
			}),
		[orderedIds, dependenciesById, pullRequestById],
	);

	const orderResult = useMemo(() => orderReviewItems(reviewItems), [reviewItems]);

	const message = useMemo(() => {
		if (!orderResult.ok) return "";
		return formatSlackMessage(orderResult.ordered, t("review.messageHeading"));
	}, [orderResult, t]);

	// Dependency options for a given PR: every other included PR.
	function dependencyOptions(id: string) {
		return orderedIds
			.filter((otherId) => otherId !== id)
			.map((otherId) => {
				const pullRequest = pullRequestById.get(otherId);
				return {
					value: otherId,
					label: pullRequest ? `#${pullRequest.number} ${pullRequest.title}` : otherId,
				};
			});
	}

	function renderBody() {
		if (loading) {
			return (
				<Group justify="center" py="lg">
					<Loader size="sm" />
				</Group>
			);
		}

		if (!connected) {
			return <Text size="sm">{t("review.notConnected")}</Text>;
		}

		if (error) {
			return (
				<Alert color="red" title={t("common.error")}>
					{t("review.loadError")}
				</Alert>
			);
		}

		if (pullRequests.length === 0) {
			return <Text size="sm">{t("review.noPullRequests")}</Text>;
		}

		return (
			<Stack gap="md">
				<Text size="sm" c="dimmed">
					{t("review.description")}
				</Text>

				<Stack gap="xs">
					{pullRequests.map((pullRequest) => {
						const id = itemId(pullRequest);
						const included = orderedIds.includes(id);
						const position = orderedIds.indexOf(id);
						return (
							<Stack
								key={id}
								gap="xs"
								p="xs"
								style={{ border: "1px solid var(--mantine-color-dark-4)", borderRadius: 4 }}
							>
								<Group justify="space-between" wrap="nowrap">
									<Checkbox
										checked={included}
										onChange={(event) => toggleInclude(id, event.currentTarget.checked)}
										label={
											<Group gap="xs" wrap="nowrap">
												<Text size="sm">
													#{pullRequest.number} {pullRequest.title}
												</Text>
												<Badge size="sm" variant="light">
													{pullRequest.repositoryName}
												</Badge>
											</Group>
										}
									/>
									{included && (
										<Group gap={4} wrap="nowrap">
											<Tooltip label={t("review.moveUp")}>
												<ActionIcon
													variant="subtle"
													size="sm"
													disabled={position <= 0}
													onClick={() => move(id, -1)}
													aria-label={t("review.moveUp")}
												>
													<ArrowUpIcon size={14} />
												</ActionIcon>
											</Tooltip>
											<Tooltip label={t("review.moveDown")}>
												<ActionIcon
													variant="subtle"
													size="sm"
													disabled={position >= orderedIds.length - 1}
													onClick={() => move(id, 1)}
													aria-label={t("review.moveDown")}
												>
													<ArrowDownIcon size={14} />
												</ActionIcon>
											</Tooltip>
										</Group>
									)}
								</Group>
								{included && (
									<MultiSelect
										size="xs"
										label={t("review.dependsOn")}
										placeholder={t("review.dependsOnPlaceholder")}
										data={dependencyOptions(id)}
										value={dependenciesById[id] ?? []}
										onChange={(value) => setDependencies(id, value)}
										searchable
										clearable
									/>
								)}
							</Stack>
						);
					})}
				</Stack>

				{!orderResult.ok && (
					<Alert color="orange" icon={<WarningIcon size={16} />} title={t("review.cycleTitle")}>
						{t("review.cycleMessage")}
					</Alert>
				)}

				{orderResult.ok && orderedIds.length > 0 && (
					<Stack gap="xs">
						<Text size="sm" fw={600}>
							{t("review.preview")}
						</Text>
						<Code block>{message}</Code>
						<Group justify="flex-end">
							<Button
								leftSection={clipboard.copied ? <CheckIcon size={16} /> : <CopyIcon size={16} />}
								color={clipboard.copied ? "green" : undefined}
								onClick={() => clipboard.copy(message)}
								disabled={!message.trim()}
							>
								{clipboard.copied ? t("common.copied") : t("review.copyMessage")}
							</Button>
						</Group>
					</Stack>
				)}
			</Stack>
		);
	}

	return (
		<Modal opened={opened} onClose={onClose} title={t("review.title")} size="lg">
			{renderBody()}
		</Modal>
	);
}
