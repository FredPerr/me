import { ActionIcon, Popover, SimpleGrid, TextInput, Tooltip } from "@mantine/core";
import { useDisclosure } from "@mantine/hooks";
import {
	BookIcon,
	BrainIcon,
	BrowserIcon,
	CircleDashedIcon,
	CloudIcon,
	CodeIcon,
	CubeIcon,
	DatabaseIcon,
	DesktopIcon,
	DeviceMobileIcon,
	FileIcon,
	FlaskIcon,
	FolderIcon,
	GearIcon,
	GitBranchIcon,
	GithubLogoIcon,
	GlobeIcon,
	type Icon,
	KeyIcon,
	LayoutIcon,
	LightningIcon,
	LockIcon,
	NotebookIcon,
	PackageIcon,
	PaintBrushIcon,
	PaletteIcon,
	RobotIcon,
	RocketIcon,
	ShieldIcon,
	TerminalIcon,
	WrenchIcon,
} from "@phosphor-icons/react";
import { useState } from "react";

type IconEntry = {
	name: string;
	component: Icon;
};

const ICON_REGISTRY: IconEntry[] = [
	{ name: "Code", component: CodeIcon },
	{ name: "Terminal", component: TerminalIcon },
	{ name: "Globe", component: GlobeIcon },
	{ name: "Database", component: DatabaseIcon },
	{ name: "Cloud", component: CloudIcon },
	{ name: "Rocket", component: RocketIcon },
	{ name: "Lightning", component: LightningIcon },
	{ name: "Cube", component: CubeIcon },
	{ name: "Package", component: PackageIcon },
	{ name: "Gear", component: GearIcon },
	{ name: "Wrench", component: WrenchIcon },
	{ name: "Flask", component: FlaskIcon },
	{ name: "Book", component: BookIcon },
	{ name: "Notebook", component: NotebookIcon },
	{ name: "Folder", component: FolderIcon },
	{ name: "File", component: FileIcon },
	{ name: "GitBranch", component: GitBranchIcon },
	{ name: "GithubLogo", component: GithubLogoIcon },
	{ name: "Desktop", component: DesktopIcon },
	{ name: "DeviceMobile", component: DeviceMobileIcon },
	{ name: "Browser", component: BrowserIcon },
	{ name: "Layout", component: LayoutIcon },
	{ name: "PaintBrush", component: PaintBrushIcon },
	{ name: "Palette", component: PaletteIcon },
	{ name: "Shield", component: ShieldIcon },
	{ name: "Lock", component: LockIcon },
	{ name: "Key", component: KeyIcon },
	{ name: "Robot", component: RobotIcon },
	{ name: "Brain", component: BrainIcon },
];

const ICON_MAP = new Map<string, Icon>(ICON_REGISTRY.map((entry) => [entry.name, entry.component]));

export function getIconComponent(name: string): Icon | null {
	return ICON_MAP.get(name) ?? null;
}

type IconPickerProps = {
	value: string;
	onChange: (value: string) => void;
};

export function IconPicker({ value, onChange }: IconPickerProps) {
	const [opened, { open, close }] = useDisclosure(false);
	const [search, setSearch] = useState("");

	const PreviewIcon = getIconComponent(value);
	const filteredIcons = ICON_REGISTRY.filter((entry) =>
		entry.name.toLowerCase().includes(search.toLowerCase()),
	);

	function handleSelect(iconName: string) {
		onChange(iconName);
		close();
	}

	return (
		<Popover opened={opened} onClose={close} position="bottom-start" width={280}>
			<Popover.Target>
				<ActionIcon variant="light" size="lg" onClick={open} aria-label="Pick icon">
					{PreviewIcon ? <PreviewIcon size={20} /> : <CircleDashedIcon size={20} />}
				</ActionIcon>
			</Popover.Target>
			<Popover.Dropdown>
				<TextInput
					placeholder="Search icons..."
					value={search}
					onChange={(e) => setSearch(e.currentTarget.value)}
					size="xs"
					mb="xs"
				/>
				<SimpleGrid cols={6} spacing={4}>
					{filteredIcons.map(({ name, component: IconComp }) => (
						<Tooltip key={name} label={name}>
							<ActionIcon
								variant={value === name ? "light" : "subtle"}
								size="md"
								onClick={() => handleSelect(name)}
								aria-label={name}
							>
								<IconComp size={18} />
							</ActionIcon>
						</Tooltip>
					))}
				</SimpleGrid>
			</Popover.Dropdown>
		</Popover>
	);
}
