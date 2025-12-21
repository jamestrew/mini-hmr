let ws: WebSocket;
const retryConnInterval: number = 2000;
const maxRetries: number = 30;
let retryCount: number = 0;

function connect() {
  try {
    ws = new WebSocket(`ws://${window.location.host}/ws`);

    ws.onopen = () => {
      console.log("[hmr] connected");
      retryCount = 0;
    };

    ws.onmessage = async (e) => {
      const payload: HotPayload = JSON.parse(e.data);
      handleMessage(payload);
    };

    ws.onerror = () => {
      // silently handle errors, onclose will trigger reconnect
    };

    ws.onclose = () => {
      if (retryCount >= maxRetries) {
        console.log("[hmr] gave up after 1min of retries");
        return;
      }
      retryCount++;
      console.log(
        `[hmr] disconnected, retrying in 2s... (${retryCount}/${maxRetries})`,
      );
      setTimeout(connect, retryConnInterval);
    };
  } catch (e) {
    if (retryCount >= maxRetries) {
      console.log("[hmr] gave up after 1min of retries");
      return;
    }
    retryCount++;
    console.log(
      `[hmr] connection failed, retrying in 2s... (${retryCount}/${maxRetries})`,
    );
    setTimeout(connect, retryConnInterval);
  }
}

const callbacks = new Map<string, (module: any) => void>();

export function accept(id: string, cb: (module: any) => void) {
  callbacks.set(id, cb);
}

type HotPayload = Connected | Ping | Update | FullReload | Error;

interface Connected {
  type: "Connected";
}

interface Ping {
  type: "Ping";
}

interface Update {
  type: "Update";
  updates: UpdatePayload[];
}

interface UpdatePayload {
  type: "JsUpdate" | "CssUpdate";
  path: string;
  timestamp: number;
}

interface FullReload {
  type: "FullReload";
}

interface Error {
  type: "Error";
}

async function handleMessage(payload: HotPayload) {
  switch (payload.type) {
    case "Connected":
      console.log("[hmr] server connected");
      break;
    case "Ping":
      // no-op
      break;
    case "Update":
      Promise.all(
        payload.updates.map(async (update) => {
          if (update.type === "JsUpdate") {
            const next = await import(`${update.path}?t=${update.timestamp}`);
            const cb = callbacks.get(update.path);
            if (cb) {
              cb(next);
            } else location.reload();
          }

          if (update.type === "CssUpdate") {
            console.log("fohgettaboudit");
          }
        }),
      );

      payload.updates.forEach((update) => {
        console.log(`[hmr] updated: ${update.path}`);
        if (update.type === "JsUpdate") {
          // const next = await import(`${update.path}?t=${update.timestamp}`);
          // console.log("[hmr] re-imported:", next);
        }
        // window.location.reload();
      });
      break;
    case "FullReload":
      window.location.reload();
      break;
    case "Error":
      console.error("[hmr] error received from server");
      break;
    default:
      console.warn("[hmr] unknown payload type:", payload);
  }
}

connect();
