import "@mantine/notifications/styles.css";
import "@/i18n";
import { MantineProvider } from "@mantine/core";
import { Notifications } from "@mantine/notifications";
import { QueryClientProvider } from "@tanstack/react-query";
import React from "react";
import ReactDOM from "react-dom/client";
import { createQueryClient } from "@/infra/queryClient";
import App from "./App";
import { ErrorModalProvider } from "./components/shared/ErrorModalProvider";
import { theme } from "./theme/theme";

const queryClient = createQueryClient();

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
	<React.StrictMode>
		<MantineProvider theme={theme} defaultColorScheme="dark">
			<Notifications position="bottom-right" />
			<QueryClientProvider client={queryClient}>
				<ErrorModalProvider>
					<App />
				</ErrorModalProvider>
			</QueryClientProvider>
		</MantineProvider>
	</React.StrictMode>,
);
