<script lang="ts">
  import * as m from "$shared/i18n/messages";
  import { preferences, type PreferenceKey } from "$domain/preferences";
  import { IS_MAC } from "$shared/platform";
  import Button from "$shared/ui/Button";
  import Select from "$shared/ui/Select";
  import Slider from "$shared/ui/Slider";
  import Switch from "$shared/ui/Switch";
  import SettingsGroup from "$shared/ui/SettingsGroup";
  import SettingsRow from "$shared/ui/SettingsRow";

  // rice fork: every rice choice can be set back here, so stock Zephium stays
  // reachable without editing rice-theme.json. A slider is shown as it is
  // dragged and written once it rests.
  let drafts = $state<Partial<Record<PreferenceKey, number>>>({});
  const sliderValue = (key: PreferenceKey) => drafts[key] ?? Number(preferences.value(key));
  function commit(key: PreferenceKey, value: number) {
    drafts[key] = value;
    void preferences.set(key, String(value)).finally(() => {
      delete drafts[key];
    });
  }

  let textured = $derived(preferences.value("ui.texture") !== "");
</script>

<SettingsGroup title={m.rice_look()} description={m.rice_look_desc()}>
  <SettingsRow
    settingId="appearance.rice-toolbar"
    title={m.rice_toolbar()}
    description={m.rice_toolbar_desc()}
  >
    <Select
      label={m.rice_toolbar()}
      labelHidden
      value={preferences.value("ui.toolbar")}
      disabled={preferences.saving()}
      options={[
        { value: "sidebar", label: m.rice_toolbar_sidebar() },
        { value: "top", label: m.rice_toolbar_top() },
      ]}
      onchange={(v) => void preferences.set("ui.toolbar", v)}
    />
  </SettingsRow>
  {#if IS_MAC}
    <SettingsRow
      settingId="appearance.rice-window-controls"
      title={m.rice_window_controls()}
      description={m.rice_window_controls_desc()}
      ><Switch
        label={m.rice_window_controls()}
        labelHidden
        checked={preferences.value("ui.window-controls") === "hover"}
        disabled={preferences.saving()}
        onchange={(v) => void preferences.set("ui.window-controls", v ? "hover" : "shown")}
      /></SettingsRow
    >
  {/if}
  <SettingsRow
    settingId="appearance.rice-radius"
    title={m.rice_radius()}
    description={m.rice_radius_desc()}
  >
    <div class="slider">
      <Slider
        label={m.rice_radius()}
        slim
        min={0}
        max={16}
        value={sliderValue("ui.radius")}
        format={(value) => `${value}px`}
        onchange={(value) => (drafts["ui.radius"] = value)}
        oncommit={(value) => commit("ui.radius", value)}
      />
    </div>
  </SettingsRow>
  {#if textured}
    <SettingsRow
      settingId="appearance.rice-texture-opacity"
      title={m.rice_texture_opacity()}
      description={m.rice_texture_opacity_desc()}
    >
      <div class="slider">
        <Slider
          label={m.rice_texture_opacity()}
          slim
          min={0}
          max={40}
          value={sliderValue("ui.texture-opacity")}
          format={(value) => `${value}%`}
          onchange={(value) => (drafts["ui.texture-opacity"] = value)}
          oncommit={(value) => commit("ui.texture-opacity", value)}
        />
      </div>
    </SettingsRow>
    <SettingsRow
      settingId="appearance.rice-texture"
      title={m.rice_texture()}
      description={m.rice_texture_desc()}
    >
      <Button
        size="compact"
        disabled={preferences.saving()}
        onclick={() => void preferences.set("ui.texture", "")}>{m.rice_texture_clear()}</Button
      >
    </SettingsRow>
  {/if}
</SettingsGroup>

<style>
  .slider {
    width: 240px;
  }
</style>
