import { AppShell, AppShellMain, AppShellNavbar } from "@mantine/core";
import { Outlet } from "react-router";
import { Navbar } from "@/components/navigation/Navbar";

export function AppLayout() {
	return (
		<AppShell
			navbar={{ width: 60, breakpoint: 0 }}
			padding="md"
			styles={{
				main: {
					background: "radial-gradient(ellipse at top right, rgba(214, 8, 103, 0.08), transparent 50%), radial-gradient(ellipse at bottom right, rgba(107, 49, 178, 0.08), transparent 50%)",
				},
				navbar: {
					background: "var(--mantine-color-dark-7)",
					borderRight: "1px solid var(--mantine-color-dark-5)",
				},
			}}
		>
			<AppShellNavbar>
				<Navbar />
			</AppShellNavbar>
			<AppShellMain>
				<Outlet />
			</AppShellMain>
		</AppShell>
	);
}
