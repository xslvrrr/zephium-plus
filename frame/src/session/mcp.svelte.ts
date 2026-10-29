import { events } from "$shared/ipc/native-events";

// rice fork: which MCP clients are connected, as native last said.
let clients = $state.raw<string[]>([]);
let error = $state<string | null>(null);
let stop: (() => void) | null = null;

export const mcp = {
  clients: () => clients,
  error: () => error,
};

export async function init() {
  if (stop) return;
  stop = () => {};
  stop = await events.uiCommand.listen((event) => {
    const id = event.payload;
    if (id.startsWith("mcp.clients=")) {
      const rest = id.slice("mcp.clients=".length);
      clients = rest === "" ? [] : rest.split("\u001f");
      error = null;
    } else if (id.startsWith("mcp.error=")) {
      error = id.slice("mcp.error=".length);
    }
  });
}
