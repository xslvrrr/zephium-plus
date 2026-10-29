import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { ZoneSpec } from "$shared/ipc/bindings";

const native = vi.hoisted(() => ({
  zones: vi.fn(async (_zones: ZoneSpec[]) => {}),
}));

vi.mock("$shared/ipc/bindings", async () => {
  const { mockBindings } = await import("$shared/testing/bindings");
  return mockBindings({ reachZones: native.zones });
});

const { setZone, zoneInside } = await import("../reach-zones.svelte");

const reach = (name: string, inside: boolean) =>
  window.dispatchEvent(new CustomEvent("zephium:reach-zone", { detail: { name, inside } }));

describe("setZone", () => {
  beforeEach(() => {
    vi.useFakeTimers({ toFake: ["requestAnimationFrame"] });
    native.zones.mockClear();
  });

  afterEach(() => {
    setZone("notch", null);
    setZone("group", null);
    vi.runAllTimers();
    vi.useRealTimers();
  });

  it("sends every zone once per frame as plain rects", () => {
    setZone("notch", {
      activate: [new DOMRect(100, 0, 200, 4)],
      keep: new DOMRect(68, 0, 264, 82),
    });
    setZone("group", { activate: [new DOMRect(0, 0, 80, 4)], keep: new DOMRect(0, 0, 104, 56) });
    expect(native.zones).not.toHaveBeenCalled();
    vi.runAllTimers();
    expect(native.zones).toHaveBeenCalledTimes(1);
    expect(native.zones).toHaveBeenCalledWith([
      {
        name: "notch",
        activate: [{ x: 100, y: 0, width: 200, height: 4 }],
        keep: { x: 68, y: 0, width: 264, height: 82 },
      },
      {
        name: "group",
        activate: [{ x: 0, y: 0, width: 80, height: 4 }],
        keep: { x: 0, y: 0, width: 104, height: 56 },
      },
    ]);
  });

  it("follows native's reach events per zone", () => {
    setZone("notch", {
      activate: [new DOMRect(100, 0, 200, 4)],
      keep: new DOMRect(68, 0, 264, 82),
    });
    expect(zoneInside("notch")).toBe(false);
    reach("notch", true);
    expect(zoneInside("notch")).toBe(true);
    expect(zoneInside("group")).toBe(false);
    reach("notch", false);
    expect(zoneInside("notch")).toBe(false);
  });
});
