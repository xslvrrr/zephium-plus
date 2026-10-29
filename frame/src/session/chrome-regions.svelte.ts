import { SvelteMap } from "svelte/reactivity";
import { commands } from "$shared/ipc/bindings";

/**
 * rice fork: the rects where the chrome takes the mouse while it is over the
 * page. Each owner (the notch, the window-controls group, a peeking card)
 * claims at most one rect; the union goes to native once per frame.
 *
 * The rects limit input only over the page: natively, outside the stage's
 * frame the chrome hit-tests as usual, since the chrome beside the page (a
 * shown sidebar) is still the chrome. In the document, while raised, the
 * stage column (`.shell > .stage-column`, exactly the page's rect) takes no
 * pointer events except on elements marked `data-chrome-region` (rice.css).
 * An owner over the page puts that attribute on the element whose rect it
 * claims, and on anything that opens from it there (menus, popovers), or
 * those stay dead to the mouse.
 */
const claims = new SvelteMap<symbol, DOMRect>();
let raised = $state(false);
let queued = false;

export const chromeRaised = () => raised;

export function claimRegion(owner: symbol, rect: DOMRect | null) {
  if (rect === null || rect.width <= 0 || rect.height <= 0) claims.delete(owner);
  else claims.set(owner, rect);
  if (queued) return;
  queued = true;
  requestAnimationFrame(() => {
    queued = false;
    const rects = [...claims.values()].map(({ x, y, width, height }) => ({ x, y, width, height }));
    raised = rects.length > 0;
    document.documentElement.toggleAttribute("data-chrome-raised", raised);
    void commands.chromeRegions(rects);
  });
}
