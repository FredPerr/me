/**
 * Returns the last segment of a path or URL.
 * Ignores trailing slashes, backslashes, and query/hash if it's a URL.
 */
export function getLastPathSegment(input: string): string {
  const segments = segmentPath(input);
  return segments[segments.length - 1] ?? "";
}

function segmentPath(input: string): string[] {
  if (!input) return [];

  let value = input.trim();

  try {
    const url = new URL(value);
    value = url.pathname;
  } catch {
    // not a valid absolute URL, treat as a plain path
  }

  // Normalize backslashes to forward slashes (Windows paths)
  value = value.replace(/\\/g, "/");

  // Strip trailing slash(es)
  value = value.replace(/\/+$/, "");

  return value.split("/");
}
