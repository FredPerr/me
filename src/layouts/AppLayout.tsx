import { AppShell, AppShellMain, AppShellNavbar } from "@mantine/core";
import { Outlet } from "react-router";
import { Navbar } from "@/components/navigation/Navbar";

export function AppLayout() {
	return (
		<AppShell navbar={{ width: 60, breakpoint: 0 }} padding="md">
			<AppShellNavbar>
				<Navbar />
			</AppShellNavbar>
			<AppShellMain>
				<Outlet />
			</AppShellMain>
		</AppShell>
	);
}
