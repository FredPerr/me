function parseUrl(value: string): URL | null {
	try {
		return new URL(value);
	} catch {
		return null;
	}
}

export function isOpenableProviderUrl(url: string, baseUrl: string): boolean {
	const target = parseUrl(url);
	const base = parseUrl(baseUrl);
	return (
		target !== null && base !== null && target.protocol === "https:" && target.host === base.host
	);
}
