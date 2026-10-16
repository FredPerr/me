import "@mantine/core/styles.css";
import "./App.css";
import { useEffect } from "react";
import { RouterProvider } from "react-router";
import { useAppUpdater } from "@/hooks/useAppUpdater";
import { useProjectChangedInvalidation } from "@/hooks/workspace/useProjectChangedInvalidation";
import { router } from "@/router";

function App() {
	const { checkForUpdates } = useAppUpdater();
	useProjectChangedInvalidation();

	useEffect(() => {
		if (import.meta.env.DEV) return;
		checkForUpdates();
	}, [checkForUpdates]);

	return <RouterProvider router={router} />;
}

export default App;
