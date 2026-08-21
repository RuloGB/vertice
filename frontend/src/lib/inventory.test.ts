import { describe, expect, it } from "vitest";
import type { Component } from "../bindings/Component";
import type { Location } from "../bindings/Location";
import { isDuplicate } from "./inventory";

function locationFixture(path: string | null): Location {
  return { path, root: "claude-skills", origin: path === null ? "embedded" : "file" };
}

function componentWithLocations(locations: Location[]): Component {
  return {
    id: "skill:fixture",
    name: "Fixture",
    kind: "skill",
    description: null,
    scope: "user",
    locations,
    provenanceHint: null,
  };
}

describe("isDuplicate", () => {
  it("is false for a component with zero locations", () => {
    expect(isDuplicate(componentWithLocations([]))).toBe(false);
  });

  it("is false for a component with a single file-backed location", () => {
    expect(isDuplicate(componentWithLocations([locationFixture("C:/skills/fixture")]))).toBe(
      false,
    );
  });

  it("is false for a component with a single nullable-path location", () => {
    expect(isDuplicate(componentWithLocations([locationFixture(null)]))).toBe(false);
  });

  it("is true only when the component has multiple locations", () => {
    const duplicated = componentWithLocations([
      locationFixture("C:/claude/skills/fixture"),
      locationFixture("C:/opencode/skills/fixture"),
    ]);

    expect(isDuplicate(duplicated)).toBe(true);
  });

  it("is true for three locations, matching the consolidated CA-3 shape", () => {
    const duplicated = componentWithLocations([
      locationFixture("C:/claude/skills/fixture"),
      locationFixture("C:/opencode/skills/fixture"),
      locationFixture("C:/copilot/skills/fixture"),
    ]);

    expect(isDuplicate(duplicated)).toBe(true);
  });

  it("counts nullable paths as locations when deciding duplication", () => {
    const duplicated = componentWithLocations([
      locationFixture("C:/claude/skills/fixture"),
      locationFixture(null),
    ]);

    expect(isDuplicate(duplicated)).toBe(true);
  });
});
