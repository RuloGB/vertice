import { describe, expect, it } from "vitest";
import type { ClientInstallSlot } from "../../bindings/ClientInstallSlot";
import type { ClientPresence } from "../../bindings/ClientPresence";
import { presenceFor } from "./presenceFor";

function record(
  slot: ClientInstallSlot,
  status: ClientPresence["status"],
  probedPath: string,
): ClientPresence {
  return {
    slot,
    label: slot,
    probedPaths: [probedPath],
    status,
    installations:
      status === "detected"
        ? [{ client: "claudeCode", version: "1.0.0", path: probedPath }]
        : [],
  };
}

describe("presenceFor — the H1 selection rule", () => {
  it("claude_code_card_reads_the_bundled_record_when_npm_is_not_detected", () => {
    const npm = record("claudeCodeNpm", "notDetected", "npm-path");
    const bundled = record("claudeCodeBundled", "detected", "bundled-path");

    const selected = presenceFor([npm, bundled], ["claudeCodeNpm", "claudeCodeBundled"]);

    expect(selected).toBe(bundled);
  });

  it("the_first_detected_record_wins_across_a_group_of_three_slots", () => {
    // Synthetic three-slot group — the real enum has no three-slot product
    // yet, but the rule must hold for any N, not just two.
    const slots = ["claudeCodeNpm", "claudeCodeBundled", "openCodeNpm"] as ClientInstallSlot[];
    const first = record(slots[0], "notDetected", "first");
    const second = record(slots[1], "notDetected", "second");
    const third = record(slots[2], "detected", "third");

    const selected = presenceFor([first, second, third], slots);

    expect(selected).toBe(third);
  });

  it("a_fully_undetected_group_still_renders_the_first_records_probed_paths", () => {
    const first = record("claudeCodeNpm", "notDetected", "first-path");
    const second = record("claudeCodeBundled", "notDetected", "second-path");

    const selected = presenceFor([first, second], ["claudeCodeNpm", "claudeCodeBundled"]);

    expect(selected).toBe(first);
    expect(selected?.probedPaths).toEqual(["first-path"]);
  });

  it("both_detected_selects_the_first_in_record_order", () => {
    // Pins the accepted Option-A limitation: a future Option C change has
    // to touch this test on purpose, not silently regress it.
    const npm = record("claudeCodeNpm", "detected", "npm-path");
    const bundled = record("claudeCodeBundled", "detected", "bundled-path");

    const selected = presenceFor([npm, bundled], ["claudeCodeNpm", "claudeCodeBundled"]);

    expect(selected).toBe(npm);
  });

  it("returns undefined for an empty group", () => {
    expect(presenceFor([], ["claudeCodeNpm"])).toBeUndefined();
  });
});
