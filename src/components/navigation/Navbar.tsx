import { ActionIcon, Stack, Tooltip } from "@mantine/core";
import { CodeIcon, GearSixIcon } from "@phosphor-icons/react";
import { useLocation, useNavigate } from "react-router";

type NavItem = {
	path: string;
	label: string;
	icon: React.ReactNode;
};

const TOP_ITEMS: NavItem[] = [
	{ path: "/", label: "Projects", icon: <CodeIcon size={24} /> },
];

const BOTTOM_ITEMS: NavItem[] = [
	{ path: "/settings", label: "Settings", icon: <GearSixIcon size={24} /> },
];

export function Navbar() {
	const navigate = useNavigate();
	const location = useLocation();

	function renderItem(item: NavItem) {
		return (
			<Tooltip key={item.path} label={item.label} position="right">
				<ActionIcon
					variant={location.pathname === item.path ? "light" : "subtle"}
					size="lg"
					onClick={() => navigate(item.path)}
					aria-label={item.label}
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
