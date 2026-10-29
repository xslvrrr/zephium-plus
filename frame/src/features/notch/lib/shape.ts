/** The new tab's notch (`newtab/lib/layout.ts`, traced by `newtab/lib/ground.ts`),
 *  which this one copies so the two read as the same cut. Restated rather than
 *  imported: a static edge into the new tab's module splits the startup
 *  bundle past its request budget. */
export const NOTCH = { width: 640, height: 50, radius: 18, flare: 14 } as const;

/** How far in from the page column's edges the notch stays. */
const INSET = 24;

const n = (value: number) => Math.round(value * 100) / 100;

export type Figure = {
  /** The whole figure, flares included: the rect it hangs in, in window points. */
  x: number;
  width: number;
  height: number;
  /** Closed along the window's top edge, to fill. */
  fill: string;
  /** The same outline left open along the top, to draw its edge. */
  edge: string;
};

/** The notch for a page column at `x` of this width, in window points,
 *  centred over it and narrowed to fit it, `depth` deep (the notch's own
 *  height, or more while something hangs under the field); null when the
 *  column has no room for its corners. Traced as `ground.ts` traces the new tab's: from the left
 *  foot on the top edge, the ground turning down into the side on the flare's
 *  radius, straight down, round the bottom corners, and back up the other
 *  side to the right foot. */
export function notchFigure(x: number, width: number, depth: number = NOTCH.height): Figure | null {
  const span = Math.max(0, Math.min(NOTCH.width, width - 2 * INSET));
  const { radius: r, flare: f } = NOTCH;
  const height = n(Math.max(NOTCH.height, depth));
  if (span < 2 * r) return null;
  const left = f;
  const right = f + span;
  const edge = [
    `M0 0`,
    `A${f} ${f} 0 0 1 ${left} ${f}`,
    `V${n(height - r)}`,
    `A${r} ${r} 0 0 0 ${n(left + r)} ${height}`,
    `H${n(right - r)}`,
    `A${r} ${r} 0 0 0 ${n(right)} ${n(height - r)}`,
    `V${f}`,
    `A${f} ${f} 0 0 1 ${n(right + f)} 0`,
  ].join("");
  return {
    x: n(x + (width - span) / 2 - f),
    width: n(span + 2 * f),
    height,
    fill: `${edge}Z`,
    edge,
  };
}
