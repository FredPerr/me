import "@mantine/core/styles.css";
import "./App.css";
import { useEffect } from "react";
import { RouterProvider } from "react-router";
import { useGitHostingChangedInvalidation } from "@/hooks/gitHosting/useGitHostingChangedInvalidation";
import { useProjectPullRequestsInvalidation } from "@/hooks/gitHosting/useProjectPullRequestsInvalidation";
import { useAppUpdater } from "@/hooks/useAppUpdater";
import { useProjectChangedInvalidation } from "@/hooks/workspace/useProjectChangedInvalidation";
import { router } from "@/router";

function App() {
	const { checkForUpdates } = useAppUpdater();
	useProjectChangedInvalidation();
	useGitHostingChangedInvalidation();
	useProjectPullRequestsInvalidation();

	useEffect(() => {
		if (import.meta.env.DEV) return;
		checkForUpdates();
	}, [checkForUpdates]);

	return <RouterProvider router={router} />;
}

export default App;
