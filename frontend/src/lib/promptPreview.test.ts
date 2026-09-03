import { describe, expect, it } from "vitest";
import { PROMPT_PREVIEW_LIMIT, promptBodyPreview } from "./promptPreview";

describe("promptBodyPreview", () => {
  it("returns the body untouched when it is within the limit", () => {
    const body = "Short prompt body";
    expect(promptBodyPreview(body)).toBe(body);
  });

  it("returns the body untouched when it is exactly at the limit", () => {
    const body = "a".repeat(PROMPT_PREVIEW_LIMIT);
    expect(promptBodyPreview(body)).toBe(body);
  });

  it("clamps to the limit and appends an ellipsis when the body is longer", () => {
    const body = "a".repeat(PROMPT_PREVIEW_LIMIT + 50);
    const preview = promptBodyPreview(body);

    expect(preview).toBe("a".repeat(PROMPT_PREVIEW_LIMIT) + "…");
    expect(preview).not.toBe(body);
  });

  it("trims trailing whitespace left by the cut before the ellipsis", () => {
    const body = "word ".repeat(PROMPT_PREVIEW_LIMIT);
    const preview = promptBodyPreview(body, 10);

    expect(preview).toBe("word word…");
  });

  it("counts by code point so surrogate pairs are not split", () => {
    const body = "😀".repeat(20);
    const preview = promptBodyPreview(body, 5);

    expect(preview).toBe("😀😀😀😀😀…");
    expect(Array.from(preview)).toHaveLength(6);
  });

  it("honors a caller-provided limit", () => {
    expect(promptBodyPreview("abcdefghij", 4)).toBe("abcd…");
  });
});
