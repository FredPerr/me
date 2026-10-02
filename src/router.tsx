import { createHashRouter } from "react-router";
import { AppLayout } from "@/layouts/AppLayout";
import { HomePage } from "@/pages/HomePage";
import { PullRequestDraftPage } from "@/pages/PullRequestDraftPage";
import { PullRequestsPage } from "@/pages/PullRequestsPage";
import { SettingsPage } from "@/pages/SettingsPage";

export const router = createHashRouter([
	{
		element: <AppLayout />,
		children: [
			{ path: "/", element: <HomePage /> },
			{ path: "/pull-requests", element: <PullRequestsPage /> },
			{ path: "/settings", element: <SettingsPage /> },
			{
				path: "/projects/:projectTag/contexts/:contextId/pull-requests",
				element: <PullRequestDraftPage />,
			},
		],
	},
]);
