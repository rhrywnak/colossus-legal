import React from "react";
import ReactDOM from "react-dom/client";
import { BrowserRouter } from "react-router-dom";
import App from "./App";
import "./styles/index.css";
// Design system tokens (colors + typography) for the Home Page Redesign (Phase 2).
// Loaded globally so every component can reference var(--token) and the type
// utility classes. Additive only — wired into components in instructions B–E.
import "./styles/tokens.css";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <BrowserRouter>
      <App />
    </BrowserRouter>
  </React.StrictMode>
);
