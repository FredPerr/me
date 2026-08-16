import "@mantine/core/styles.css";
import "./App.css";
import { AppShell, AppShellMain } from "@mantine/core";
import { ProjectTabs } from "./components/project-tabs/ProjectTabs";
import type { Project } from "./models/Project";

const PROJECTS: Project[] = [
  {
    name: "Targipsum",
    tag: "targi",
  },
  {
    name: "dotfiles",
    tag: "dotfiles",
  },
];

function App() {
  return (
    <AppShell>
      <AppShellMain>
        <ProjectTabs projects={PROJECTS} />
      </AppShellMain>
    </AppShell>
  );
}

export default App;
