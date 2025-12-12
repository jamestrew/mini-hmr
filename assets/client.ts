const ws = new WebSocket(`ws://${window.location.host}/ws`);

ws.onopen = () => console.log("connected");
ws.onmessage = (e) => console.log("message:", e.data);
ws.onclose = () => console.log("disconnected");
ws.onerror = (e) => console.error("ws error:", e);
