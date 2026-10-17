/** Up to two uppercase initials from a display name, for an avatar fallback. */
export function initialsOf(displayName: string): string {
	return displayName
		.split(/\s+/)
		.filter((part) => part.length > 0)
		.slice(0, 2)
		.map((part) => part[0]?.toUpperCase())
		.join("");
}
