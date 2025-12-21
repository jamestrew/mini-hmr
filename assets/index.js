import { accept } from "/hmr-client.js";

export function renderApp() {
	const root = document.getElementById("root");
	root.innerHTML = "";

	const div = document.createElement("div");

	div.textContent = `Rendered at: ${new Date().toLocaleTimeString()}`;
	root.appendChild(div);
}

function renderOther() {
  const other = document.getElementById("other");
  other.innerHTML = "";
  const p = document.createElement("p");
  p.textContent = `Other content rendered at: ${new Date().toLocaleTimeString()}`;
  other.appendChild(p);
}

// Only run renderOther on initial load, not on HMR
if (!window.__hmr_initialized) {
	window.__hmr_initialized = true;
	renderOther();
}

renderApp();

accept("/assets/index.js", (module) => {
	module.renderApp();
});
