import { describe, it, expect } from "vitest";
import { getLastPathSegment } from "./segmentPath";


describe("getLastPathSegment", () => {
  it("returns an empty string when input is empty", () => {
    const input = "";
    const expected = "";

    const result = getLastPathSegment(input);

    expect(result).toBe(expected);
  });

  it("returns the file name from a unix-style path", () => {
    const input = "/foo/bar/baz.txt";
    const expected = "baz.txt";

    const result = getLastPathSegment(input);

    expect(result).toBe(expected);
  });

  it("returns the file name from a windows-style path", () => {
    const input = "foo\\bar\\baz.txt";
    const expected = "baz.txt";

    const result = getLastPathSegment(input);

    expect(result).toBe(expected);
  });

  it("returns the last directory name when the path has a trailing slash", () => {
    const input = "/foo/bar/";
    const expected = "bar";

    const result = getLastPathSegment(input);

    expect(result).toBe(expected);
  });

  it("returns the last directory name when the path has multiple trailing slashes", () => {
    const input = "/foo/bar///";
    const expected = "bar";

    const result = getLastPathSegment(input);

    expect(result).toBe(expected);
  });

  it("returns the file name from an absolute URL, ignoring query and hash", () => {
    const input = "https://site.com/a/b/file.txt?x=1#section";
    const expected = "file.txt";

    const result = getLastPathSegment(input);

    expect(result).toBe(expected);
  });

  it("returns the domain-relative root segment for a bare host URL", () => {
    const input = "https://site.com";
    const expected = "";

    const result = getLastPathSegment(input);

    expect(result).toBe(expected);
  });

  it("returns the file name for a single-segment relative path", () => {
    const input = "baz.txt";
    const expected = "baz.txt";

    const result = getLastPathSegment(input);

    expect(result).toBe(expected);
  });

  it("trims surrounding whitespace before extracting the last segment", () => {
    const input = "  /foo/bar/baz.txt  ";
    const expected = "baz.txt";

    const result = getLastPathSegment(input);

    expect(result).toBe(expected);
  });
});