export function renderApp() {
  const root = document.getElementById("root");
  root.innerHTML = "";

  const div = document.createElement("div");
  div.textContent = `Rendered at: ${new Date().toLocaleTimeString()}`;
  root.appendChild(div);
}

renderApp();
