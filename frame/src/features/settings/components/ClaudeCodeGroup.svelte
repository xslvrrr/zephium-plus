<script lang="ts">
  import * as m from "$shared/i18n/messages";
  import { preferences } from "$domain/preferences";
  import { commands, type ClaudeCodeStatus } from "$shared/ipc/bindings";
  import ProviderMark from "$shared/ui/ProviderMark";
  import SettingsGroup from "$shared/ui/SettingsGroup";
  import Switch from "$shared/ui/Switch";

  // rice fork: Ask can be answered by Claude Code, headless, which acts on
  // the browser through Zephium's own MCP server. Choosing it also turns on
  // Claude access, which that server is.
  let status = $state<ClaudeCodeStatus | null>(null);
  $effect(() => {
    void commands
      .claudeCodeStatus()
      .then((next) => (status = next))
      .catch(() => {});
  });

  const chosen = $derived(preferences.value("ai.claude-code") === "true");
  const missing = $derived(status !== null && (status.path === null || !status.bridge));
  const detail = $derived.by(() => {
    if (status === null) return m.claude_code_desc();
    if (status.path === null) return m.claude_code_missing();
    if (!status.bridge) return m.claude_code_no_bridge();
    return m.claude_code_found({ path: status.path });
  });

  async function choose(on: boolean) {
    if (on && preferences.value("mcp.enabled") !== "true")
      await preferences.set("mcp.enabled", "true");
    await preferences.set("ai.claude-code", String(on));
  }
</script>

<SettingsGroup title={m.claude_code_group()} description={m.claude_code_desc()}>
  <div class="claude-row">
    <ProviderMark provider="anthropic" size={32} />
    <div class="copy">
      <h3>{m.claude_code_title()}</h3>
      <p>{detail}</p>
    </div>
    <Switch
      label={m.claude_code_title()}
      labelHidden
      checked={chosen}
      disabled={preferences.saving() || (missing && !chosen)}
      onchange={(on) => void choose(on)}
    />
  </div>
</SettingsGroup>

<style>
  /* Metrics match AiProviderRow so the page reads as one list. */
  .claude-row {
    display: flex;
    align-items: center;
    gap: 12px;
    box-sizing: border-box;
    min-height: var(--row-page);
    padding: 12px 18px 12px 14px;
  }

  .copy {
    flex: 1;
    min-width: 0;
  }

  h3 {
    margin: 0;
    color: var(--color-text);
    font-size: var(--text-page-title);
    font-weight: 500;
    line-height: 19px;
    letter-spacing: -0.008em;
  }

  p {
    margin: 2px 0 0;
    color: var(--color-muted);
    font-size: var(--text-label);
    line-height: 1.45;
    overflow-wrap: anywhere;
  }
</style>
