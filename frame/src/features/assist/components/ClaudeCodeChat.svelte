<script lang="ts">
  import { tick } from "svelte";
  import * as m from "$shared/i18n/messages";
  import { tabs } from "$domain/tabs";
  import Icon from "$shared/ui/Icon";
  import { claudeChat } from "../lib/claude-code.svelte";
  import { ArrowUp02Icon, StopIcon } from "../lib/icons";
  import TextBlock from "./blocks/TextBlock.svelte";

  /** rice fork: Ask answered by Claude Code, headless, acting on the browser
   *  through Zephium's MCP server (each action still asks here first). */
  let { space }: { space: string | null } = $props();

  const chat = $derived(claudeChat(space));
  let draft = $state("");
  let field = $state<HTMLTextAreaElement>();
  let list = $state<HTMLElement>();

  const page = $derived.by(() => {
    const tab = tabs.activeTab();
    if (!tab?.url) return null;
    return tab.title ? `${tab.title} — ${tab.url}` : tab.url;
  });

  /** Readable tool names: `mcp__zephium__read_page` reads as "read page". */
  const toolName = (name: string) =>
    name
      .replace(/^mcp__zephium__/u, "")
      .replace(/^mcp__/u, "")
      .replaceAll("_", " ");

  async function send() {
    const text = draft.trim();
    if (!text || chat.running !== null) return;
    draft = "";
    await chat.ask(text, page);
    await tick();
    list?.scrollTo({ top: list.scrollHeight });
  }

  $effect(() => {
    void chat.turns;
    void tick().then(() => list?.scrollTo({ top: list.scrollHeight }));
  });

  export function focus() {
    field?.focus();
  }
</script>

<div class="claude-chat">
  <div class="turns" bind:this={list} data-assist-thread>
    {#if chat.turns.length === 0}
      <p class="empty">{m.claude_code_empty()}</p>
    {/if}
    {#each chat.turns as turn, index (index)}
      <div class="turn">
        <p class="ask">{turn.ask}</p>
        {#if turn.tools.length}
          <ul class="tools" aria-label={m.claude_code_tools()}>
            {#each turn.tools as tool, at (at)}<li>{toolName(tool)}</li>{/each}
          </ul>
        {/if}
        {#if turn.answer}
          <div class="answer">
            <TextBlock text={turn.answer} cites={[]} oncite={() => Promise.resolve(false)} />
          </div>
        {:else if !turn.done}
          <p class="working" role="status">{m.claude_code_working()}</p>
        {/if}
        {#if turn.error}<p class="error" role="alert">{turn.error}</p>{/if}
      </div>
    {/each}
  </div>
  <form
    class="composer"
    onsubmit={(event) => {
      event.preventDefault();
      void send();
    }}
  >
    <textarea
      bind:this={field}
      bind:value={draft}
      rows="2"
      placeholder={m.claude_code_placeholder()}
      aria-label={m.claude_code_placeholder()}
      onkeydown={(event) => {
        if (event.key === "Enter" && !event.shiftKey && !event.isComposing) {
          event.preventDefault();
          void send();
        } else if (event.key === "Escape" && chat.running !== null) {
          event.preventDefault();
          event.stopPropagation();
          chat.stop();
        }
      }}></textarea>
    {#if chat.running !== null}
      <button
        type="button"
        class="send"
        aria-label={m.claude_code_stop()}
        onclick={() => chat.stop()}><Icon icon={StopIcon} size={14} /></button
      >
    {:else}
      <button type="submit" class="send" aria-label={m.claude_code_send()} disabled={!draft.trim()}
        ><Icon icon={ArrowUp02Icon} size={14} /></button
      >
    {/if}
  </form>
</div>

<style>
  .claude-chat {
    display: flex;
    flex: 1;
    flex-direction: column;
    min-block-size: 0;
  }

  .turns {
    display: flex;
    flex: 1;
    flex-direction: column;
    gap: 16px;
    min-block-size: 0;
    overflow-y: auto;
    padding: 12px 14px;
  }

  .empty,
  .working {
    margin: 0;
    color: var(--color-muted);
    font-size: var(--text-label);
  }

  .turn {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .ask {
    align-self: flex-end;
    max-inline-size: 85%;
    margin: 0;
    padding: 6px 10px;
    border-radius: var(--radius-control);
    background: var(--color-fill-hover);
    color: var(--color-text);
    font-size: var(--text-body);
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }

  .tools {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .tools li {
    padding: 1px 6px;
    border-radius: var(--radius-capsule);
    background: var(--color-fill-hover);
    color: var(--color-muted);
    font-size: var(--text-label);
  }

  .answer {
    color: var(--color-text);
    font-size: var(--text-body);
  }

  .error {
    margin: 0;
    color: var(--color-danger);
    font-size: var(--text-label);
    overflow-wrap: anywhere;
  }

  .composer {
    display: flex;
    gap: 6px;
    align-items: flex-end;
    margin: 0 10px 10px;
    padding: 6px 6px 6px 10px;
    border-radius: var(--radius-card);
    background: var(--color-address);
    box-shadow: var(--shadow-tile);
  }

  textarea {
    flex: 1;
    min-inline-size: 0;
    border: 0;
    background: transparent;
    color: var(--color-text);
    font: inherit;
    font-size: var(--text-body);
    resize: none;
    outline: none;
  }

  .send {
    display: grid;
    place-items: center;
    inline-size: 26px;
    block-size: 26px;
    padding: 0;
    border: 0;
    border-radius: var(--radius-capsule);
    background: var(--color-lit);
    color: var(--color-on-lit);
    cursor: default;
  }

  .send:disabled {
    background: var(--color-fill-strong);
  }

  .send:focus-visible,
  textarea:focus-visible {
    outline: 2px solid var(--color-ring);
    outline-offset: 2px;
  }

  .send:hover:not(:disabled) {
    background: var(--color-lit-hover);
  }
</style>
