import { createBrowserRouter } from "react-router";
import { AppLayout } from "@/layouts/AppLayout";
import { HomePage } from "@/pages/HomePage";
import { SettingsPage } from "@/pages/SettingsPage";

export const router = createBrowserRouter([
	{
		element: <AppLayout />,
		children: [
			{ path: "/", element: <HomePage /> },
			{ path: "/settings", element: <SettingsPage /> },
		],
	},
]);
