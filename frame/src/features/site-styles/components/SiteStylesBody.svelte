<script lang="ts">
  import * as m from "$shared/i18n/messages";
  import { preferences } from "$domain/preferences";
  import { commands, type SiteStylesChange, type SiteStylesView } from "$shared/ipc/bindings";
  import Button from "$shared/ui/Button";
  import Switch from "$shared/ui/Switch";

  /** rice fork: the site's styles on or off, each of its features, or, for a
   *  site with no style, forced transparency. Changes apply to open pages at
   *  once. */
  let { host, open }: { host: string; open: boolean } = $props();

  let view = $state<SiteStylesView | null>(null);
  let error = $state<string | null>(null);
  let busy = $state(false);
  let transparency = $derived(
    preferences.value("ui.page-transparency") === "true" &&
      preferences.value("ui.material") !== "none",
  );

  $effect(() => {
    if (!open) return;
    const asked = host;
    error = null;
    void commands
      .siteStylesFor(asked)
      .then((next) => {
        if (asked === host) view = next;
      })
      .catch(() => (view = null));
  });

  async function change(next: SiteStylesChange) {
    busy = true;
    try {
      const result = await commands.siteStylesSet(host, next);
      if (result.status === "ok") {
        view = result.data;
        error = null;
      } else {
        error = result.error;
      }
    } finally {
      busy = false;
    }
  }
</script>

{#if view}
  <div class="styles" role="group" aria-label={m.site_styles_title()}>
    <div class="row">
      <span class="copy">
        <span class="title">{m.site_styles_title()}</span>
        <span class="desc">{view.styled ? m.site_styles_by() : m.site_styles_none()}</span>
      </span>
      <Switch
        label={m.site_styles_title()}
        labelHidden
        checked={view.enabled}
        disabled={busy}
        onchange={(on) => void change({ kind: "enabled", on })}
      />
    </div>
    {#if view.enabled && view.styled}
      {#each view.features as feature (feature.key)}
        {@const held = feature.transparency && !transparency}
        <div class="row">
          <span class="copy">
            <span class="title">{feature.title}</span>
            {#if held}
              <span class="desc">{m.site_styles_needs_transparency()}</span>
            {:else if feature.description}
              <span class="desc">{feature.description}</span>
            {/if}
          </span>
          <Switch
            label={feature.title}
            labelHidden
            checked={feature.on}
            disabled={busy || held}
            onchange={(on) => void change({ kind: "feature", key: feature.key, on })}
          />
        </div>
      {/each}
    {:else if view.enabled}
      <div class="row">
        <span class="copy">
          <span class="title">{m.site_styles_force()}</span>
          <span class="desc"
            >{transparency ? m.site_styles_force_desc() : m.site_styles_needs_transparency()}</span
          >
        </span>
        <Switch
          label={m.site_styles_force()}
          labelHidden
          checked={view.forced && transparency}
          disabled={busy || !transparency}
          onchange={(on) => void change({ kind: "forced", on })}
        />
      </div>
    {/if}
    {#if view.changed}
      <div class="reset">
        <Button size="compact" disabled={busy} onclick={() => void change({ kind: "reset" })}
          >{m.site_styles_reset()}</Button
        >
      </div>
    {/if}
    {#if error}
      <p class="desc" role="status">{error}</p>
    {/if}
  </div>
{/if}

<style>
  .styles {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-inline-size: 0;
  }

  .row {
    display: flex;
    align-items: center;
    gap: 12px;
    min-block-size: var(--menu-row);
    padding-inline: var(--menu-item-inset);
  }

  .copy {
    display: flex;
    flex: 1 1 auto;
    flex-direction: column;
    min-inline-size: 0;
  }

  .title {
    color: var(--color-text);
    font-size: var(--text-body);
    font-weight: 450;
    overflow-wrap: anywhere;
  }

  .desc {
    margin: 0;
    color: var(--color-muted);
    font-size: var(--text-label);
    line-height: 1.4;
    overflow-wrap: anywhere;
  }

  p.desc {
    padding-inline: var(--menu-item-inset);
  }

  .reset {
    display: flex;
    justify-content: flex-end;
    padding-inline: var(--menu-item-inset);
  }
</style>
