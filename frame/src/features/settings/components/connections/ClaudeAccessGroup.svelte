<script lang="ts">
  import * as m from "$shared/i18n/messages";
  import { preferences } from "$domain/preferences";
  import Button from "$shared/ui/Button";
  import SettingsGroup from "$shared/ui/SettingsGroup";
  import SettingsRow from "$shared/ui/SettingsRow";
  import Switch from "$shared/ui/Switch";
  import { mcp } from "$session/mcp.svelte";

  // rice fork: the bridge binary ships inside the app bundle.
  const COMMAND =
    "claude mcp add zephium -- /Applications/Zephium.app/Contents/MacOS/zephium-mcp";
  let enabled = $derived(preferences.value("mcp.enabled") === "true");
  let allowed = $derived(
    preferences
      .value("mcp.allowed")
      .split("\n")
      .filter((name) => name !== ""),
  );
  let copied = $state(false);
  let manual = $state(false);
  let field = $state<HTMLInputElement>();

  async function copy() {
    try {
      await navigator.clipboard.writeText(COMMAND);
      copied = true;
      manual = false;
    } catch {
      // No clipboard access here: show the command selected so it can be copied by hand.
      manual = true;
      queueMicrotask(() => field?.select());
    }
  }
  function revoke(name: string) {
    void preferences.set("mcp.allowed", allowed.filter((n) => n !== name).join("\n"));
  }
</script>

<SettingsGroup title={m.mcp_group()} description={m.mcp_group_note()}>
  <SettingsRow
    settingId="connections.claude-access"
    title={m.mcp_enabled()}
    description={m.mcp_enabled_help()}
  >
    <Switch
      label={m.mcp_enabled()}
      labelHidden
      checked={enabled}
      disabled={preferences.saving()}
      onchange={(v) => void preferences.set("mcp.enabled", String(v))}
    />
  </SettingsRow>
  {#if enabled}
    <SettingsRow title={m.mcp_setup()} description={COMMAND}>
      <Button size="compact" onclick={() => void copy()}
        >{copied ? m.mcp_copied() : m.mcp_copy()}</Button
      >
    </SettingsRow>
    {#if manual}
      <input
        bind:this={field}
        class="mcp-command"
        readonly
        value={COMMAND}
        aria-label={m.mcp_setup()}
        onfocus={(event) => event.currentTarget.select()}
      />
    {/if}
    {#if mcp.error()}<p class="mcp-error" role="alert">{m.mcp_socket_error()}</p>{/if}
  {/if}
  {#each allowed as name (name)}
    <SettingsRow title={name} description={m.mcp_always_allowed()}>
      <Button size="compact" onclick={() => revoke(name)}>{m.mcp_revoke()}</Button>
    </SettingsRow>
  {/each}
</SettingsGroup>

<style>
  .mcp-command {
    box-sizing: border-box;
    inline-size: calc(100% - 36px);
    margin: 0 18px 12px;
    padding: 6px 8px;
    border: 0;
    border-radius: var(--radius-inset);
    background: var(--color-fill);
    color: var(--color-text);
    font: inherit;
    font-size: var(--text-label);
  }

  .mcp-error {
    margin: 0;
    padding: 0 18px 12px;
    color: var(--color-danger);
    font-size: var(--text-label);
  }
</style>
