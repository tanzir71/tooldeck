import { isTauri } from "@tauri-apps/api/core";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import { App } from "./App";
import { MenuBarPanel } from "./components/MenuBarPanel";
import "./styles.css";

const surface = isTauri()
  ? getCurrentWebviewWindow().label
  : new URLSearchParams(window.location.search).get("surface");

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    {surface === "menu-bar" ? <MenuBarPanel /> : <App />}
  </StrictMode>,
);
