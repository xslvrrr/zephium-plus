import { SvelteMap } from "svelte/reactivity";
import { commands } from "$shared/ipc/bindings";

type Spec = { activate: DOMRect[]; keep: DOMRect };
const specs = new SvelteMap<string, Spec>();
const inside = $state<Record<string, boolean>>({});
let queued = false;
let listening = false;
/** When the chrome's own document last took a press. */
let pressedAt = -Infinity;
/** A native press with no press in the document this close to it landed
 *  outside the document: on the page. */
const PRESS_PAIRING_MS = 250;

const plain = ({ x, y, width, height }: DOMRect) => ({ x, y, width, height });

/** rice fork: whether the pointer is in a named zone (native watches it, so
 *  it is known over the page as well as over the chrome). */
export const zoneInside = (name: string) => inside[name] === true;

export function setZone(name: string, spec: Spec | null) {
  if (!listening) {
    listening = true;
    window.addEventListener("zephium:reach-zone", (event) => {
      const { name: zone, inside: now } = (event as CustomEvent<{ name: string; inside: boolean }>)
        .detail;
      // Native repeats a held zone whenever the zones are sent again.
      if (inside[zone] !== now) inside[zone] = now;
    });
    // rice fork: a press on the page never reaches this document, so what
    // closes on a press elsewhere (the notch's field, an open menu) stayed
    // open. Native reports every press in the window; one the document did
    // not also take is handed to it as a press outside everything.
    window.addEventListener(
      "pointerdown",
      (event) => {
        if (event.isTrusted) pressedAt = performance.now();
      },
      true,
    );
    window.addEventListener("zephium:reach-press", () => {
      const native = performance.now();
      setTimeout(() => {
        if (Math.abs(pressedAt - native) < PRESS_PAIRING_MS) return;
        document.body.dispatchEvent(
          new PointerEvent("pointerdown", { bubbles: true, cancelable: true, composed: true }),
        );
      }, PRESS_PAIRING_MS);
    });
  }
  if (spec === null) specs.delete(name);
  else specs.set(name, spec);
  if (queued) return;
  queued = true;
  requestAnimationFrame(() => {
    queued = false;
    void commands.reachZones(
      [...specs].map(([zone, { activate, keep }]) => ({
        name: zone,
        activate: activate.map(plain),
        keep: plain(keep),
      })),
    );
  });
}
