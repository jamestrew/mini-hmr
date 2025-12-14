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

    ws.onmessage = (e) => console.log("[hmr] message:", e.data);

    ws.onerror = () => {
      // silently handle errors, onclose will trigger reconnect
    };

    ws.onclose = () => {
      if (retryCount >= maxRetries) {
        console.log("[hmr] gave up after 1min of retries");
        return;
      }
      retryCount++;
      console.log(`[hmr] disconnected, retrying in 2s... (${retryCount}/${maxRetries})`);
      setTimeout(connect, retryConnInterval);
    };
  } catch (e) {
    if (retryCount >= maxRetries) {
      console.log("[hmr] gave up after 1min of retries");
      return;
    }
    retryCount++;
    console.log(`[hmr] connection failed, retrying in 2s... (${retryCount}/${maxRetries})`);
    setTimeout(connect, retryConnInterval);
  }
}

connect();
