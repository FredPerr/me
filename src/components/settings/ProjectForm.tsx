import {
  ActionIcon,
  Button,
  CopyButton,
  Group,
  Stack,
  TextInput,
  Tooltip,
} from "@mantine/core";
import { CheckIcon, CopyIcon } from "@phosphor-icons/react";
import { useEffect, useState } from "react";
import type { Project, SubProject } from "@/models/Project";
import { ProjectDirectory } from "@/models/ProjectDirectory";
import { resolvePath } from "@/utils/resolvePath";
import { SubProjectFormList } from "./SubProjectFormList";

type ProjectFormProps = {
  initialProject?: Project;
  onSubmit: (project: Project) => void;
  onCancel: () => void;
};

export function ProjectForm({
  initialProject,
  onSubmit,
  onCancel,
}: ProjectFormProps) {
  const [name, setName] = useState(initialProject?.name ?? "");
  const [tag, setTag] = useState(initialProject?.tag ?? "");
  const [path, setPath] = useState(initialProject?.path ?? "");
  const [subprojects, setSubprojects] = useState<SubProject[]>(
    initialProject?.subprojects ?? [],
  );
  const [configPath, setConfigPath] = useState("");

  const isEditing = !!initialProject;

  useEffect(() => {
    if (initialProject) {
      ProjectDirectory.getConfigFilePath(initialProject.tag).then(
        setConfigPath,
      );
    }
  }, [initialProject]);

  async function handleSubmit(event: React.SubmitEvent) {
    event.preventDefault();
    const resolvedPath = await resolvePath(path, {
      isFolder: true,
    });
    onSubmit({
      name,
      tag,
      path: resolvedPath,
      subprojects,
    });
  }

  return (
    <form onSubmit={handleSubmit}>
      <Stack gap="sm">
        <TextInput
          label="Name"
          placeholder="My Project"
          value={name}
          onChange={(event) => setName(event.currentTarget.value)}
          required
        />
        <TextInput
          label="Tag"
          placeholder="my-project"
          value={tag}
          onChange={(event) => setTag(event.currentTarget.value)}
          disabled={isEditing}
          required
        />
        <TextInput
          label="Path"
          placeholder="~/Projects/my-project"
          value={path}
          onChange={(event) => setPath(event.currentTarget.value)}
          required
        />
        <SubProjectFormList
          subprojects={subprojects}
          onChange={setSubprojects}
        />
        {isEditing && configPath && (
          <TextInput
            label="Config path"
            readOnly
            disabled
            value={configPath}
            rightSection={
              <CopyButton value={configPath}>
                {({ copied, copy }) => (
                  <Tooltip label={copied ? "Copied" : "Copy config path"}>
                    <ActionIcon
                      variant="transparent"
                      size="xs"
                      onClick={copy}
                      aria-label="Copy config path"
                    >
                      {copied ? (
                        <CheckIcon size={12} />
                      ) : (
                        <CopyIcon size={12} />
                      )}
                    </ActionIcon>
                  </Tooltip>
                )}
              </CopyButton>
            }
          />
        )}
        <Group justify="flex-end">
          <Button variant="subtle" onClick={onCancel}>
            Cancel
          </Button>
          <Button type="submit">{isEditing ? "Save" : "Add"}</Button>
        </Group>
      </Stack>
    </form>
  );
}
