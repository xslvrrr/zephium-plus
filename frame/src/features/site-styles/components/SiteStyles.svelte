<script lang="ts">
  import * as m from "$shared/i18n/messages";
  import { preferences } from "$domain/preferences";

  /** rice fork: Zen Internet's switches for the page's site, loaded the
   *  first time the panel is opened; a failed load is tried again at the
   *  next. Nothing shows while site styles are off or the page has no web
   *  host. */
  let { page, open }: { page: { url: string } | null; open: boolean } = $props();

  let Body = $state.raw<typeof import("./SiteStylesBody.svelte").default | null>(null);
  let failed = $state(false);
  let loading = false;

  let host = $derived.by(() => {
    if (!page) return null;
    try {
      const url = new URL(page.url);
      return url.protocol === "http:" || url.protocol === "https:" ? url.hostname : null;
    } catch {
      return null;
    }
  });
  let shown = $derived(host !== null && preferences.value("site-styles.enabled") === "true");

  $effect(() => {
    if (!open || !shown || Body !== null || loading) return;
    loading = true;
    failed = false;
    void import("./SiteStylesBody.svelte")
      .then((module) => (Body = module.default))
      .catch(() => (failed = true))
      .finally(() => (loading = false));
  });
</script>

{#if shown && host && Body}
  <Body {host} {open} />
{:else if shown && failed}
  <p role="status">{m.surface_render_failed()}</p>
{/if}

<style>
  p {
    margin: 0;
    padding-inline: var(--menu-item-inset);
    color: var(--color-muted);
    font-size: var(--text-label);
  }
</style>
