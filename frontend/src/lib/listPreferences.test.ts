// @vitest-environment jsdom

import { beforeEach, describe, expect, it } from "vitest";
import { getListPageSize, setListPageSize } from "./listPreferences";

describe("listPreferences", () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it("persists each list's selected page size independently across a new read", () => {
    setListPageSize("skills", 15);
    setListPageSize("prompts", 10);

    expect(getListPageSize("skills")).toBe(15);
    expect(getListPageSize("prompts")).toBe(10);
    expect(getListPageSize("agents")).toBe(5);
  });

  it("falls back safely when storage contains an unsupported value", () => {
    localStorage.setItem("vertice.list-page-size.skills", "999");

    expect(getListPageSize("skills")).toBe(5);
  });
});
