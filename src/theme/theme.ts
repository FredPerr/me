import { createTheme, type MantineColorsTuple } from "@mantine/core";

const primary: MantineColorsTuple = [
	"#ffe9f6",
	"#ffd1e6",
	"#faa1c9",
	"#f66eab",
	"#f24391",
	"#f02981",
	"#f01879",
	"#d60867",
	"#c0005c",
	"#a9004f",
];

const secondary: MantineColorsTuple = [
	"#f6eeff",
	"#e7d9f7",
	"#cab1ea",
	"#ad86dd",
	"#9462d2",
	"#854bcb",
	"#7d3fc9",
	"#6b31b2",
	"#5f2ba0",
	"#52238d",
];

export const theme = createTheme({
	colors: {
		primary,
		secondary,
	},
	primaryColor: "primary",
});
