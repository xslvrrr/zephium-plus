<script lang="ts">
  import * as m from "$shared/i18n/messages";
  import { preferences } from "$domain/preferences";
  import { commands, type SiteStylesStatus } from "$shared/ipc/bindings";
  import Button from "$shared/ui/Button";
  import Switch from "$shared/ui/Switch";
  import SettingsGroup from "$shared/ui/SettingsGroup";
  import SettingsRow from "$shared/ui/SettingsRow";

  // rice fork: page transparency and Zen Internet's site styles. Each site's
  // own switches are in its site menu.
  let status = $state<SiteStylesStatus | null>(null);
  let updating = $state(false);

  const when = new Intl.DateTimeFormat(undefined, { dateStyle: "medium", timeStyle: "short" });
  const on = (key: Parameters<typeof preferences.value>[0]) => preferences.value(key) === "true";

  let solid = $derived(preferences.value("ui.material") === "none");
  let styled = $derived(on("site-styles.enabled"));
  let statusText = $derived.by(() => {
    if (!status) return m.site_styles_auto_update_desc();
    if (status.error) return m.site_styles_status_error({ error: status.error });
    const sites = status.sites;
    return status.updated === null
      ? m.site_styles_status_bundled({ sites })
      : m.site_styles_status({ sites, when: when.format(new Date(status.updated)) });
  });

  $effect(() => {
    void commands
      .siteStylesStatus()
      .then((next) => (status = next))
      .catch(() => {});
  });

  async function updateNow() {
    updating = true;
    try {
      status = await commands.siteStylesUpdateNow();
    } catch {
      // The status keeps what it said before.
    } finally {
      updating = false;
    }
  }
</script>

<SettingsGroup title={m.site_styles_group()} description={m.site_styles_group_desc()}>
  <SettingsRow
    settingId="appearance.page-transparency"
    title={m.rice_page_transparency()}
    description={solid ? m.rice_page_transparency_solid() : m.rice_page_transparency_desc()}
    ><Switch
      label={m.rice_page_transparency()}
      labelHidden
      checked={on("ui.page-transparency")}
      disabled={preferences.saving() || solid}
      onchange={(v) => void preferences.set("ui.page-transparency", String(v))}
    /></SettingsRow
  >
  <SettingsRow
    settingId="appearance.site-styles"
    title={m.site_styles_title()}
    description={m.site_styles_desc()}
    ><Switch
      label={m.site_styles_title()}
      labelHidden
      checked={styled}
      disabled={preferences.saving()}
      onchange={(v) => void preferences.set("site-styles.enabled", String(v))}
    /></SettingsRow
  >
  <SettingsRow
    settingId="appearance.site-styles-force"
    title={m.site_styles_force_all()}
    description={m.site_styles_force_all_desc()}
    ><Switch
      label={m.site_styles_force_all()}
      labelHidden
      checked={on("site-styles.force")}
      disabled={preferences.saving() || !styled}
      onchange={(v) => void preferences.set("site-styles.force", String(v))}
    /></SettingsRow
  >
  <SettingsRow
    settingId="appearance.site-styles-update"
    title={m.site_styles_auto_update()}
    description={statusText}
  >
    <div class="update">
      <Button size="compact" disabled={updating} onclick={() => void updateNow()}
        >{updating ? m.site_styles_updating() : m.site_styles_update_now()}</Button
      >
      <Switch
        label={m.site_styles_auto_update()}
        labelHidden
        checked={preferences.value("site-styles.auto-update") !== "false"}
        disabled={preferences.saving() || !styled}
        onchange={(v) => void preferences.set("site-styles.auto-update", String(v))}
      />
    </div>
  </SettingsRow>
</SettingsGroup>

<style>
  .update {
    display: flex;
    align-items: center;
    gap: 12px;
  }
</style>
