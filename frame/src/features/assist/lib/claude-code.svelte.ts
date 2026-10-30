import { commands } from "$shared/ipc/bindings";

/** rice fork: Ask answered by Claude Code, headless, through Zephium's own
 *  MCP server. One conversation per Space, held for the session; Claude Code
 *  keeps the conversation itself and is resumed by its session id. */

export type ClaudeTurn = {
  ask: string;
  answer: string;
  tools: string[];
  error: string | null;
  run: number | null;
  done: boolean;
};

type ClaudeEvent =
  | { kind: "started"; run: number; session: string }
  | { kind: "text"; run: number; text: string }
  | { kind: "tool"; run: number; name: string }
  | {
      kind: "done";
      run: number;
      session: string | null;
      error: string | null;
      cost: number | null;
    };

export class ClaudeChat {
  turns = $state<ClaudeTurn[]>([]);
  running = $state<number | null>(null);
  session: string | null = null;

  /** Asks, with the tab in front named for context. */
  async ask(text: string, page: string | null) {
    const ask = text.trim();
    if (!ask || this.running !== null) return;
    const turn: ClaudeTurn = { ask, answer: "", tools: [], error: null, run: null, done: false };
    this.turns = [...this.turns, turn];
    const at = this.turns.length - 1;
    const result = await commands.claudeCodeAsk(ask, this.session, page);
    if (result.status === "ok") {
      this.running = result.data;
      this.update(at, { run: result.data });
      runs.set(result.data, this);
    } else {
      this.update(at, { error: result.error, done: true });
    }
  }

  stop() {
    if (this.running !== null) void commands.claudeCodeCancel(this.running);
  }

  clear() {
    this.stop();
    this.turns = [];
    this.session = null;
  }

  private update(at: number, change: Partial<ClaudeTurn>) {
    this.turns = this.turns.map((turn, index) => (index === at ? { ...turn, ...change } : turn));
  }

  receive(event: ClaudeEvent) {
    const at = this.turns.findIndex((turn) => turn.run === event.run);
    if (at < 0) return;
    const turn = this.turns[at]!;
    switch (event.kind) {
      case "started":
        this.session = event.session;
        break;
      case "text":
        this.update(at, { answer: turn.answer ? `${turn.answer}\n\n${event.text}` : event.text });
        break;
      case "tool":
        this.update(at, { tools: [...turn.tools, event.name] });
        break;
      case "done":
        if (event.session) this.session = event.session;
        this.update(at, { error: event.error, done: true });
        if (this.running === event.run) this.running = null;
        runs.delete(event.run);
        break;
    }
  }
}

// Lookups, not state: a chat is created while a view derives it.
// eslint-disable-next-line svelte/prefer-svelte-reactivity
const runs = new Map<number, ClaudeChat>();
// eslint-disable-next-line svelte/prefer-svelte-reactivity
const chats = new Map<string, ClaudeChat>();
let listening = false;

/** The conversation of a Space (or of the window, with none). */
export function claudeChat(space: string | null): ClaudeChat {
  if (!listening) {
    listening = true;
    window.addEventListener("zephium:claude-code", (event) => {
      const detail = (event as CustomEvent<ClaudeEvent>).detail;
      runs.get(detail.run)?.receive(detail);
    });
  }
  const key = space ?? "";
  let chat = chats.get(key);
  if (!chat) {
    chat = new ClaudeChat();
    chats.set(key, chat);
  }
  return chat;
}
