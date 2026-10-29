import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { RegionRect } from "$shared/ipc/bindings";

const native = vi.hoisted(() => ({
  regions: vi.fn(async (_rects: RegionRect[]) => {}),
}));

vi.mock("$shared/ipc/bindings", async () => {
  const { mockBindings } = await import("$shared/testing/bindings");
  return mockBindings({ chromeRegions: native.regions });
});

const { chromeRaised, claimRegion } = await import("../chrome-regions.svelte");

const notch = Symbol("notch");
const group = Symbol("group");

describe("claimRegion", () => {
  beforeEach(() => {
    vi.useFakeTimers({ toFake: ["requestAnimationFrame"] });
    native.regions.mockClear();
  });

  afterEach(() => {
    claimRegion(notch, null);
    claimRegion(group, null);
    vi.runAllTimers();
    vi.useRealTimers();
  });

  it("sends the union of claims once per frame and raises the chrome", () => {
    claimRegion(notch, new DOMRect(100, 0, 200, 32));
    claimRegion(group, new DOMRect(0, 0, 80, 32));
    expect(native.regions).not.toHaveBeenCalled();
    vi.runAllTimers();
    expect(native.regions).toHaveBeenCalledTimes(1);
    expect(native.regions).toHaveBeenCalledWith([
      { x: 100, y: 0, width: 200, height: 32 },
      { x: 0, y: 0, width: 80, height: 32 },
    ]);
    expect(chromeRaised()).toBe(true);
    expect(document.documentElement.hasAttribute("data-chrome-raised")).toBe(true);
  });

  it("lowers the chrome once every claim is released or empty", () => {
    claimRegion(notch, new DOMRect(100, 0, 200, 32));
    vi.runAllTimers();
    claimRegion(notch, new DOMRect(100, 0, 0, 32));
    vi.runAllTimers();
    expect(native.regions).toHaveBeenLastCalledWith([]);
    expect(chromeRaised()).toBe(false);
    expect(document.documentElement.hasAttribute("data-chrome-raised")).toBe(false);
  });
});
