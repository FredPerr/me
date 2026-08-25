import {
  ActionIcon,
  Button,
  Group,
  Modal,
  Stack,
  TextInput,
  Title,
} from "@mantine/core";
import { useDisclosure } from "@mantine/hooks";
import { PlusIcon, XIcon } from "@phosphor-icons/react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { useProjects } from "@/hooks/useProjects";
import type { Project } from "@/models/Project";
import { ProjectDirectory } from "@/models/ProjectDirectory";
import { ProjectForm } from "./ProjectForm";
import { ProjectList } from "./ProjectList";

export function ProjectSettings() {
  const { t } = useTranslation();
  const { projects, reload } = useProjects();
  const [editingProject, setEditingProject] = useState<Project | undefined>();
  const [opened, { open, close }] = useDisclosure(false);
  const [searchFilter, setSearchFilter] = useState("");

  function handleAdd() {
    setEditingProject(undefined);
    open();
  }

  function handleEdit(project: Project) {
    setEditingProject(project);
    open();
  }

  async function handleDelete(project: Project) {
    await ProjectDirectory.deleteProject(project.tag);
    await reload();
  }

  async function handleSubmit(project: Project) {
    await ProjectDirectory.saveProject(project);
    close();
    await reload();
  }

  return (
    <Stack gap="lg">
      <Group justify="space-between">
        <Title order={3}>{t("settings.projects.title")}</Title>
        <Group>
          <TextInput
            value={searchFilter}
            placeholder={t("settings.projects.searchPlaceholder")}
            onChange={(e) => {
              setSearchFilter(e.currentTarget.value);
            }}
            rightSection={
              <ActionIcon
                variant="transparent"
                onClick={() => {
                  setSearchFilter("");
                }}
              >
                <XIcon size={10} />
              </ActionIcon>
            }
          />

          <Button leftSection={<PlusIcon />} onClick={handleAdd}>
            {t("settings.projects.addProject")}
          </Button>
        </Group>
      </Group>

      <ProjectList
        projects={projects}
        onEdit={handleEdit}
        onDelete={handleDelete}
        searchValue={searchFilter}
      />

      <Modal
        opened={opened}
        onClose={close}
        size="xl"
        title={editingProject ? t("settings.projects.editProject") : t("settings.projects.addProject")}
      >
        <ProjectForm
          initialProject={editingProject}
          onSubmit={handleSubmit}
          onCancel={close}
        />
      </Modal>
    </Stack>
  );
}
