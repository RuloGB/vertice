import { describe, expect, it } from "vitest";
import type { Component } from "../bindings/Component";
import type { ComponentKind } from "../bindings/ComponentKind";
import { filterComponents } from "./filterComponents";

/** Build a fully-typed `Component` fixture with a single file-backed location. */
function componentFixture(id: string, name: string, kind: ComponentKind): Component {
  return {
    id,
    name,
    kind,
    description: null,
    scope: "user",
    locations: [{ path: `C:/fixtures/${id}`, root: "claude-skills", origin: "file" }],
    provenanceHint: null,
  };
}

function mixedComponents(): Component[] {
  return [
    componentFixture("skill:formatter", "Formatter", "skill"),
    componentFixture("skill:test-runner", "Test Runner", "skill"),
    componentFixture("agent:form-wizard", "Form Wizard", "agent"),
    componentFixture("agent:release-manager", "Release Manager", "agent"),
  ];
}

describe("filterComponents", () => {
  it("returns every component unchanged for the all kind and an empty query", () => {
    const components = mixedComponents();

    const result = filterComponents(components, { kind: "all", query: "" });

    expect(result).toEqual(components);
  });

  it("keeps only skill components when the kind filter is skill", () => {
    const result = filterComponents(mixedComponents(), { kind: "skill", query: "" });

    expect(result.map((component) => component.id)).toEqual([
      "skill:formatter",
      "skill:test-runner",
    ]);
  });

  it("keeps only agent components when the kind filter is agent", () => {
    const result = filterComponents(mixedComponents(), { kind: "agent", query: "" });

    expect(result.map((component) => component.id)).toEqual([
      "agent:form-wizard",
      "agent:release-manager",
    ]);
  });

  it("matches name queries case-insensitively against component names", () => {
    const result = filterComponents(mixedComponents(), { kind: "all", query: "form" });

    expect(result.map((component) => component.id)).toEqual([
      "skill:formatter",
      "agent:form-wizard",
    ]);
  });

  it("matches regardless of the query's own casing", () => {
    const result = filterComponents(mixedComponents(), { kind: "all", query: "FORMATTER" });

    expect(result.map((component) => component.id)).toEqual(["skill:formatter"]);
  });

  it("combines the kind filter and the name query", () => {
    const result = filterComponents(mixedComponents(), { kind: "skill", query: "form" });

    expect(result.map((component) => component.id)).toEqual(["skill:formatter"]);
  });

  it("returns an empty list when the query matches no component", () => {
    const result = filterComponents(mixedComponents(), { kind: "all", query: "no-such-name" });

    expect(result).toEqual([]);
  });

  it("returns an empty list for an empty report", () => {
    const result = filterComponents([], { kind: "skill", query: "form" });

    expect(result).toEqual([]);
  });

  it("does not mutate the source array or its entries", () => {
    const components = mixedComponents();
    const snapshot = structuredClone(components);

    filterComponents(components, { kind: "skill", query: "form" });

    expect(components).toEqual(snapshot);
  });
});
