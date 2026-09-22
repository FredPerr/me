import {
	ActionIcon,
	Alert,
	Anchor,
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
import {
	inferDependencies,
	moveItem,
	orderReviewItems,
	type ReviewItem,
} from "@/models/review-share/ReviewStack";
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

	// Selection state: the ordered list of included PR ids, and per-PR manual
	// dependency overrides. An entry of `undefined` means "use the dependencies
	// inferred from the base branch"; a set value means the user overrode them.
	const [orderedIds, setOrderedIds] = useState<string[]>([]);
	const [overridesById, setOverridesById] = useState<Record<string, string[] | undefined>>({});

	const pullRequestById = useMemo(() => {
		const map = new Map<string, ReviewPullRequest>();
		for (const pullRequest of pullRequests) {
			map.set(itemId(pullRequest), pullRequest);
		}
		return map;
	}, [pullRequests]);

	// Dependencies inferred from branch relationships across all PRs, computed
	// once. In a stack, a PR's base branch is the head branch of the PR beneath
	// it, so this reconstructs the stack without any manual input.
	const inferredDependencies = useMemo(
		() =>
			inferDependencies(
				pullRequests.map((pullRequest) => ({
					id: itemId(pullRequest),
					headBranch: pullRequest.headBranch,
					baseBranch: pullRequest.baseBranch,
				})),
			),
		[pullRequests],
	);

	const includedIds = useMemo(() => new Set(orderedIds), [orderedIds]);

	// Effective dependencies for a PR: the manual override if set, otherwise the
	// inferred ones — always narrowed to PRs that are currently included.
	const effectiveDependencies = useMemo(() => {
		const result: Record<string, string[]> = {};
		for (const id of orderedIds) {
			const source = overridesById[id] ?? inferredDependencies[id] ?? [];
			result[id] = source.filter((dependencyId) => includedIds.has(dependencyId));
		}
		return result;
	}, [orderedIds, overridesById, inferredDependencies, includedIds]);

	function isOverridden(id: string): boolean {
		return overridesById[id] !== undefined;
	}

	function toggleInclude(id: string, included: boolean) {
		setOrderedIds((current) =>
			included ? [...current, id] : current.filter((existing) => existing !== id),
		);
		if (!included) {
			// Drop any manual override for the excluded PR and remove it from
			// every other PR's override so no stale reference survives.
			setOverridesById((current) => {
				const next: Record<string, string[] | undefined> = {};
				for (const [key, deps] of Object.entries(current)) {
					if (key === id || deps === undefined) continue;
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
		setOverridesById((current) => ({ ...current, [id]: dependencyIds }));
	}

	function resetToInferred(id: string) {
		setOverridesById((current) => ({ ...current, [id]: undefined }));
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
						dependsOn: effectiveDependencies[id] ?? [],
					},
				];
			}),
		[orderedIds, effectiveDependencies, pullRequestById],
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
										label={
											<Group gap="xs" justify="space-between" wrap="nowrap">
												<Text size="xs" fw={500}>
													{t("review.dependsOn")}
												</Text>
												{isOverridden(id) ? (
													<Anchor size="xs" onClick={() => resetToInferred(id)}>
														{t("review.resetToAuto")}
													</Anchor>
												) : (
													<Text size="xs" c="dimmed">
														{t("review.autoFromBase")}
													</Text>
												)}
											</Group>
										}
										placeholder={t("review.dependsOnPlaceholder")}
										data={dependencyOptions(id)}
										value={effectiveDependencies[id] ?? []}
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
