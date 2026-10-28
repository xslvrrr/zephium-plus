<script lang="ts">
  import { mediaUrl } from "$domain/resources";
  import { preferences } from "$domain/preferences";
  import { tabs } from "$domain/tabs";

  // One bounded decode at the window's own scale; never re-requested on resize.
  const width = Math.min(1600, Math.ceil(window.screen.width * window.devicePixelRatio));
  let src = $derived.by(() => {
    const digest = preferences.value("ui.texture");
    const profile = tabs.profile()?.id;
    if (!profile || !/^[0-9a-f]{64}$/u.test(digest)) return null;
    return mediaUrl(profile, digest, width);
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
