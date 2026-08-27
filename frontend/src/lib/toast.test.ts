import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import * as toast from "./toast.svelte";

beforeEach(() => {
  vi.useFakeTimers();
  toast.clearAll();
});

afterEach(() => {
  toast.clearAll();
  vi.useRealTimers();
});

describe("toast store", () => {
  it("adds a success toast with a unique id", () => {
    toast.success("Done");

    const toasts = toast.getToasts();
    expect(toasts).toHaveLength(1);
    expect(toasts[0].kind).toBe("success");
    expect(toasts[0].message).toBe("Done");
  });

  it("adds error and warning toasts with distinct kinds", () => {
    toast.error("Failed");
    toast.warning("Careful");

    const toasts = toast.getToasts();
    expect(toasts).toHaveLength(2);
    expect(toasts[0].kind).toBe("error");
    expect(toasts[1].kind).toBe("warning");
  });

  it("assigns unique incrementing ids to each toast", () => {
    toast.success("First");
    toast.success("Second");

    const [first, second] = toast.getToasts();
    expect(first.id).not.toBe(second.id);
  });

  it("auto-dismisses after the default duration", () => {
    toast.success("Gone soon");
    expect(toast.getToasts()).toHaveLength(1);

    vi.advanceTimersByTime(4000);
    expect(toast.getToasts()).toHaveLength(0);
  });

  it("does not auto-dismiss before the default duration", () => {
    toast.success("Still here");

    vi.advanceTimersByTime(3999);
    expect(toast.getToasts()).toHaveLength(1);
  });

  it("dismisses a specific toast by id", () => {
    toast.success("Keep");
    toast.error("Remove me");
    const [, second] = toast.getToasts();

    toast.dismiss(second.id);

    const remaining = toast.getToasts();
    expect(remaining).toHaveLength(1);
    expect(remaining[0].message).toBe("Keep");
  });

  it("clearAll removes every toast and cancels pending timers", () => {
    toast.success("One");
    toast.error("Two");
    toast.warning("Three");

    toast.clearAll();
    expect(toast.getToasts()).toHaveLength(0);

    vi.advanceTimersByTime(10_000);
    expect(toast.getToasts()).toHaveLength(0);
  });

  it("dismiss is a no-op for unknown ids", () => {
    toast.success("Safe");

    toast.dismiss(9999);

    expect(toast.getToasts()).toHaveLength(1);
  });

  it("keeps independent auto-dismiss timers per toast", () => {
    toast.success("First");
    vi.advanceTimersByTime(1000);
    toast.success("Second");

    vi.advanceTimersByTime(3000);
    const remaining = toast.getToasts();
    expect(remaining).toHaveLength(1);
    expect(remaining[0].message).toBe("Second");

    vi.advanceTimersByTime(1000);
    expect(toast.getToasts()).toHaveLength(0);
  });
});
