import "@mantine/core/styles.css";
import "./App.css";
import { AppShell, AppShellMain, Button, Tabs, TabsList, TabsPanel } from "@mantine/core";
import { ChatCircleIcon, GearSixIcon, ImageIcon } from "@phosphor-icons/react";

function App() {
	return (
		<AppShell>
			<AppShellMain>
				<Tabs defaultValue={"gallery"}>
					<Tabs.List>
						<Tabs.Tab value="gallery" leftSection={<ImageIcon size={12} />}>
							Gallery
						</Tabs.Tab>
						<Tabs.Tab value="messages" leftSection={<ChatCircleIcon size={12} />}>
							Messages
						</Tabs.Tab>
						<Tabs.Tab value="settings" leftSection={<GearSixIcon size={12} />}>
							Settings
						</Tabs.Tab>
					</Tabs.List>
					<Tabs.Panel value="gallery">Gallery tab content</Tabs.Panel>
					<Tabs.Panel value="messages">Messages tab content</Tabs.Panel>
					<Tabs.Panel value="settings">Settings tab content</Tabs.Panel>
				</Tabs>
			</AppShellMain>
		</AppShell>
		// <AppShell>
		//   <AppShellMain>
		//       <Tabs defaultValue={"gallery"}>
		//   <Tabs.List>
		//     <Tabs.Tab value="gallery" leftSection={<ImageIcon size={12} />}>
		//       Gallery
		//     </Tabs.Tab>
		//     <Tabs.Tab value="messages" leftSection={<ChatCircleIcon size={12} />}>
		//       Messages
		//     </Tabs.Tab>
		//     <Tabs.Tab value="settings" leftSection={<GearSixIcon size={12} />}>
		//       Settings
		//     </Tabs.Tab>
		//   </Tabs.List>

		//   <Tabs.Panel value="gallery">
		//     Gallery tab content
		//   </Tabs.Panel>

		//   <Tabs.Panel value="messages">
		//     Messages tab content
		//   </Tabs.Panel>

		//   <Tabs.Panel value="settings">
		//     Settings tab content
		//   </Tabs.Panel>
		// </Tabs>
		//       </Tabs>
		//   </AppShellMain>
		// </AppShell>
	);
}

export default App;
