import { describe, expect, it } from "vitest";
import type { Location } from "../bindings/Location";
import { CLIENT_LABEL, groupLocationsByClient } from "./clientGroups";

function location(client: Location["client"]): Location {
  return {
    path: "/some/path",
    root: "some-root",
    origin: "file",
    mcpTransport: null,
    client,
  };
}

describe("groupLocationsByClient", () => {
  it("deduplicates locations by client and counts them", () => {
    const locations: Location[] = [
      location("claudeCode"),
      location("claudeCode"),
      location("openCode"),
    ];

    const groups = groupLocationsByClient(locations);

    expect(groups).toEqual([
      { client: "claudeCode", count: 2 },
      { client: "openCode", count: 1 },
    ]);
  });

  it("returns groups in fixed order regardless of input order", () => {
    const locations: Location[] = [
      location("codex"),
      location(null),
      location("claudeCode"),
      location("openCode"),
    ];

    const groups = groupLocationsByClient(locations);

    expect(groups.map((g) => g.client)).toEqual([
      "claudeCode",
      "openCode",
      "codex",
      null,
    ]);
  });

  it("places shared (null) last", () => {
    const locations: Location[] = [location(null), location("claudeCode")];

    const groups = groupLocationsByClient(locations);

    expect(groups[groups.length - 1].client).toBeNull();
    expect(groups[0].client).toBe("claudeCode");
  });

  it("returns an empty array for empty input", () => {
    expect(groupLocationsByClient([])).toEqual([]);
  });

  it("omits clients with zero locations", () => {
    const locations: Location[] = [location("codex")];

    const groups = groupLocationsByClient(locations);

    expect(groups).toEqual([{ client: "codex", count: 1 }]);
    expect(groups.find((g) => g.client === "claudeCode")).toBeUndefined();
    expect(groups.find((g) => g.client === "openCode")).toBeUndefined();
  });

  it("handles a mix of all clients and shared", () => {
    const locations: Location[] = [
      location("claudeCode"),
      location("openCode"),
      location("openCode"),
      location("codex"),
      location(null),
      location("claudeCode"),
      location("claudeCode"),
    ];

    const groups = groupLocationsByClient(locations);

    expect(groups).toEqual([
      { client: "claudeCode", count: 3 },
      { client: "openCode", count: 2 },
      { client: "codex", count: 1 },
      { client: null, count: 1 },
    ]);
  });
});

describe("CLIENT_LABEL", () => {
  it("maps each ClientKind to its proper noun", () => {
    expect(CLIENT_LABEL.claudeCode).toBe("Claude Code");
    expect(CLIENT_LABEL.openCode).toBe("OpenCode");
    expect(CLIENT_LABEL.codex).toBe("Codex");
  });
});
