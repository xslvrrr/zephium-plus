<script lang="ts">
  import wave from "$shared/brand/wave-light.png";

  // Where the pointer is within the mark's own box: -1 at one edge, 0 at the
  // centre, 1 at the other. `over` holds while the pointer is inside the box,
  // so the mark answers to being touched rather than to being near.
  let px = $state(0);
  let py = $state(0);
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
      const over = Math.abs(x) <= 1 && Math.abs(y) <= 1;
      px = over ? x : 0;
      py = over ? y : 0;
    });
  }
  $effect(() => {
    const root = document.documentElement;
    const observer = new MutationObserver(() => {
      if (reduce()) onpointerleave();
    });
    observer.observe(root, { attributes: true, attributeFilter: ["data-reduce-motion"] });
    return () => observer.disconnect();
  });
  function onpointerleave() {
    px = 0;
    py = 0;
  }
</script>

<svelte:window {onpointermove} />
<svelte:document {onpointerleave} />

{#snippet layers()}
  <div class="layers">
    <div class="ramp"></div>
    <div class="streak"></div>
  </div>
{/snippet}

<div class="rice-mark" aria-hidden="true">
  <div
    class="plate"
    style:--px={px}
    style:--py={py}
    style:--wave={`url(${wave})`}
    bind:this={plate}
  >
    <div class="glow">{@render layers()}</div>
    <img src={wave} alt="" draggable="false" />
    <div class="sheen">{@render layers()}</div>
  </div>
</div>

<style>
  /* Values ported from Trance (trance-tokens.css, trance-surfaces.css). */
  .rice-mark {
    --mark-size: min(28vmin, 250px);
    --mark-alpha: 0.35;
    --mark-tilt: 14deg;
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

  /* The sheen is the mark's own shape: the art masks a clipped box holding two
     oversized layers (oil-slick ramp and a specular comb) screened together.
     The pointer moves the layers by transform only; the travel equals Trance's
     background-position parallax (50% of the layer range, at its per-axis
     rates), expressed as a share of each layer's own size. */
  .sheen,
  .glow {
    position: absolute;
    inset: 0;
  }

  .layers {
    position: absolute;
    inset: 0;
    overflow: hidden;
    isolation: isolate;
    mask: var(--wave) center / contain no-repeat;
  }

  .ramp,
  .streak {
    position: absolute;
    transition: transform var(--motion-base) var(--ease-out);
  }

  .ramp {
    inset: -200%;
    background-image: var(--mark-sheen);
    transform: translate(calc(var(--px) * -40%), calc(var(--py) * 30%));
  }

  .streak {
    inset: -100%;
    background-image: var(--mark-streak);
    mix-blend-mode: screen;
    transform: translate(calc(var(--px) * 33.333%), calc(var(--py) * -16.667%));
  }

  .sheen {
    opacity: var(--mark-sheen-alpha);
  }

  /* A blurred copy of the sheen behind the mark. The blur is on the box and the
     mask on the inner layers, because filter runs before mask. */
  .glow {
    z-index: -1;
    filter: blur(var(--mark-glow-blur));
    opacity: calc(var(--mark-glow-alpha) * var(--mark-sheen-alpha));
  }

  :global([data-reduce-motion="true"]) .plate,
  :global([data-reduce-motion="true"]) .ramp,
  :global([data-reduce-motion="true"]) .streak {
    transition: none;
  }
</style>
