import { ActionIcon, TextInput } from "@mantine/core";
import { XIcon } from "@phosphor-icons/react";

type SearchContextInputProps = {
	value: string;
	onChange: (value: string) => void;
};

export function SearchContextInput({ value, onChange }: SearchContextInputProps) {
	return (
		<TextInput
			value={value}
			placeholder="Search a context"
			onChange={(e) => onChange(e.currentTarget.value)}
			size="xs"
			rightSection={
				value && (
					<ActionIcon variant="transparent" size="xs" onClick={() => onChange("")}>
						<XIcon size={10} />
					</ActionIcon>
				)
			}
		/>
	);
}
