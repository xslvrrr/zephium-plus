<script lang="ts">
  import { preferences } from "$domain/preferences";
  import { tabs } from "$domain/tabs";

  // One bounded decode at the window's own scale; never re-requested on resize.
  const width = Math.min(1600, Math.ceil(window.screen.width * window.devicePixelRatio));
  let src = $state<string | null>(null);
  // The resources barrel carries the task session, so it loads only once a
  // texture is set, keeping it out of the browser's startup graph.
  $effect(() => {
    const digest = preferences.value("ui.texture");
    const profile = tabs.profile()?.id;
    if (!profile || !/^[0-9a-f]{64}$/u.test(digest)) {
      src = null;
      return;
    }
    let current = true;
    void import("$domain/resources").then(({ mediaUrl }) => {
      if (current) src = mediaUrl(profile, digest, width);
    });
    return () => {
      current = false;
    };
  });
  let failed = $state<string | null>(null);
</script>

{#if src && failed !== src}
  <img
    class="rice-texture"
    {src}
    alt=""
    aria-hidden="true"
    decoding="async"
    onerror={() => (failed = src)}
  />
{/if}

<style>
  .rice-texture {
    position: fixed;
    inset: 0;
    inline-size: 100%;
    block-size: 100%;
    object-fit: cover;
    opacity: var(--rice-texture-opacity);
    pointer-events: none;

    /* Under the chrome's own controls, over its ground. */
    z-index: 0;
    mix-blend-mode: normal;
  }

  @media (forced-colors: active) {
    .rice-texture {
      display: none;
    }
  }
</style>
