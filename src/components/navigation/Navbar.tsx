import { ActionIcon, Stack, Tooltip } from "@mantine/core";
import { CodeIcon, GearSixIcon, GitPullRequestIcon } from "@phosphor-icons/react";
import { useTranslation } from "react-i18next";
import { useLocation, useNavigate } from "react-router";

type NavItem = {
	path: string;
	labelKey: string;
	icon: React.ReactNode;
};

const TOP_ITEMS: NavItem[] = [
	{ path: "/", labelKey: "navigation.projects", icon: <CodeIcon size={24} /> },
	{
		path: "/pull-requests",
		labelKey: "navigation.pullRequests",
		icon: <GitPullRequestIcon size={24} />,
	},
];

const BOTTOM_ITEMS: NavItem[] = [
	{ path: "/settings", labelKey: "navigation.settings", icon: <GearSixIcon size={24} /> },
];

export function Navbar() {
	const { t } = useTranslation();
	const navigate = useNavigate();
	const location = useLocation();

	function renderItem(item: NavItem) {
		const label = t(item.labelKey);
		return (
			<Tooltip key={item.path} label={label} position="right">
				<ActionIcon
					variant={location.pathname === item.path ? "light" : "subtle"}
					size="lg"
					onClick={() => navigate(item.path)}
					aria-label={label}
				>
					{item.icon}
				</ActionIcon>
			</Tooltip>
		);
	}

	return (
		<Stack justify="space-between" align="center" py="md" h="100%">
			<Stack gap="xs" align="center">
				{TOP_ITEMS.map(renderItem)}
			</Stack>
			<Stack gap="xs" align="center">
				{BOTTOM_ITEMS.map(renderItem)}
			</Stack>
		</Stack>
	);
}
