import { Tabs, TabsList } from "@mantine/core";
import type { Project } from "@/models/Project";
import { ProjectTab } from "./ProjectTab";
import { ProjectTabPanel } from "./ProjectTabPanel";

type ProjectTabsProps = {
  projects: Project[];
};

export function ProjectTabs({ projects }: ProjectTabsProps) {
  const firstProject = projects[0];

  return (
    <Tabs defaultValue={firstProject.tag}>
      <TabsList>
        {projects.map((project) => (
          <ProjectTab
            displayName={project.name}
            projectTag={project.tag}
            icon={project.icon}
            key={project.tag}
          />
        ))}
      </TabsList>
      {projects.map((project) => (
        <ProjectTabPanel project={project} key={project.tag} />
      ))}
    </Tabs>
  );
}
