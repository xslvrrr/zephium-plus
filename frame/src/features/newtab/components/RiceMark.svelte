<script lang="ts">
  import wave from "$shared/brand/wave-light.png";

  // Where the pointer is within the mark's own box: -1 at one edge, 0 at the
  // centre, 1 at the other. `over` holds while the pointer is inside the box,
  // so the mark answers to being touched rather than to being near.
  let px = $state(0);
  let py = $state(0);
  let over = $state(false);
  let plate: HTMLDivElement | undefined;
  let frame = 0;
  const reduce = () => document.documentElement.getAttribute("data-reduce-motion") === "true";

  function onpointermove(event: PointerEvent) {
    if (reduce() || frame || !plate) return;
    const { clientX, clientY } = event;
    frame = requestAnimationFrame(() => {
      frame = 0;
      if (!plate) return;
      const box = plate.getBoundingClientRect();
      const x = ((clientX - box.left) / box.width - 0.5) * 2;
      const y = ((clientY - box.top) / box.height - 0.5) * 2;
      over = Math.abs(x) <= 1 && Math.abs(y) <= 1;
      px = over ? x : 0;
      py = over ? y : 0;
    });
  }
  function onpointerleave() {
    over = false;
    px = 0;
    py = 0;
  }
</script>

<svelte:window {onpointermove} />
<svelte:document {onpointerleave} />

<div class="rice-mark" aria-hidden="true">
  <div
    class="plate"
    class:over
    style:--px={px}
    style:--py={py}
    style:--wave={`url(${wave})`}
    bind:this={plate}
  >
    <div class="glow"></div>
    <img src={wave} alt="" draggable="false" />
    <div class="sheen"></div>
  </div>
</div>

<style>
  /* Values ported from Trance (trance-tokens.css, trance-surfaces.css). */
  .rice-mark {
    --mark-size: min(28vmin, 250px);
    --mark-alpha: 0.35;
    --mark-tilt: 14deg;
    --mark-parallax: 50%;
    --mark-sheen-alpha: 0.7;
    --mark-glow-blur: 24px;
    --mark-glow-alpha: 0.8;
    --mark-sheen: var(--rice-mark-sheen);
    --mark-streak: var(--rice-mark-streak);

    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    pointer-events: none;
  }

  /* perspective() lives in the element's own transform list so no ancestor
     becomes a containing block. X is inverted: a pointer below centre tips the
     bottom towards the viewer. */
  .plate {
    --sheen-position:
      calc(50% + var(--px) * var(--mark-parallax) * -1)
        calc(50% + var(--py) * var(--mark-parallax) * 0.5),
      calc(50% + var(--px) * var(--mark-parallax))
        calc(50% + var(--py) * var(--mark-parallax) * -0.75);

    position: relative;
    inline-size: var(--mark-size);
    aspect-ratio: 1;
    transform: perspective(calc(var(--mark-size) * 2))
      rotateX(calc(var(--py) * var(--mark-tilt) * -1)) rotateY(calc(var(--px) * var(--mark-tilt)));
    transition: transform var(--motion-base) var(--ease-out);
  }

  img {
    position: absolute;
    inset: 0;
    inline-size: 100%;
    block-size: 100%;
    object-fit: contain;
    opacity: var(--mark-alpha);
  }

  /* The sheen is the mark's own shape: two layers (oil-slick ramp and a
     specular comb) screened together, masked by the art. */
  .sheen,
  .glow::before {
    content: "";
    position: absolute;
    inset: 0;
    background-image: var(--mark-streak), var(--mark-sheen);
    background-repeat: no-repeat, no-repeat;
    background-size:
      300% 300%,
      500% 500%;
    background-position:
      center center,
      center center;
    background-blend-mode: screen;
    mask: var(--wave) center / contain no-repeat;
  }

  .sheen {
    opacity: var(--mark-sheen-alpha);
    transition: background-position var(--motion-base) var(--ease-out);
  }

  .over .sheen,
  .over .glow::before {
    background-position: var(--sheen-position);
  }

  /* A blurred copy of the sheen behind the mark. The blur is on the box and the
     mask on the pseudo-element, because filter runs before mask. */
  .glow {
    position: absolute;
    z-index: -1;
    inset: 0;
    filter: blur(var(--mark-glow-blur));
    opacity: calc(var(--mark-glow-alpha) * var(--mark-sheen-alpha));
  }

  :global([data-reduce-motion="true"]) .plate,
  :global([data-reduce-motion="true"]) .sheen {
    transition: none;
  }
</style>
