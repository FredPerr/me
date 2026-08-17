import { ActionIcon, Stack, Tooltip } from "@mantine/core";
import { FolderSimpleIcon, GearSixIcon } from "@phosphor-icons/react";
import { useLocation, useNavigate } from "react-router";

type NavItem = {
	path: string;
	label: string;
	icon: React.ReactNode;
};

const NAV_ITEMS: NavItem[] = [
	{ path: "/", label: "Projects", icon: <FolderSimpleIcon size={24} /> },
	{ path: "/settings", label: "Settings", icon: <GearSixIcon size={24} /> },
];

export function Navbar() {
	const navigate = useNavigate();
	const location = useLocation();

	return (
		<Stack gap="xs" align="center" py="md">
			{NAV_ITEMS.map((item) => (
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
			))}
		</Stack>
	);
}
