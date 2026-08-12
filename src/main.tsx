import React from "react";
import ReactDOM from "react-dom/client";
import { getCurrentWindow } from "@tauri-apps/api/window";
import App from "./App";
import { AlertOverlay } from "./components/alert/AlertOverlay";
import "./styles/globals.css";

/**
 * Both the tray popup and the alert overlay are served from this bundle;
 * the native window label decides which one mounts.
 */
function resolveWindowLabel(): string {
  try {
    return getCurrentWindow().label;
  } catch {
    return "main";
  }
}

const label = resolveWindowLabel();
document.documentElement.dataset.window = label;

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    {label === "notification" ? <AlertOverlay /> : <App />}
  </React.StrictMode>,
);
