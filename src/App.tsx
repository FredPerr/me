import "@mantine/core/styles.css";
import "./App.css";
import { useEffect } from "react";
import { RouterProvider } from "react-router";
import { useAppUpdater } from "@/hooks/useAppUpdater";
import { router } from "@/router";

function App() {
	const { checkForUpdates } = useAppUpdater();

	useEffect(() => {
		checkForUpdates();
	}, [checkForUpdates]);

	return <RouterProvider router={router} />;
}

export default App;
