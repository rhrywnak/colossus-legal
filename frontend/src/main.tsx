import React from "react";
import ReactDOM from "react-dom/client";
import { BrowserRouter } from "react-router-dom";
import App from "./App";
// Inter, SELF-HOSTED (task 2.13c). Imported here rather than linked from the
// Google Fonts CDN in index.html, which is what this replaces.
//
// ## Why self-hosting is not a preference
//
// A CDN <link> is a third-party request on every page load of a litigation tool:
// it leaks that this browser opened this app to a party with no role in the
// case, it fails closed on a restricted network (the font silently falls back to
// system-ui, which is Roman's "font sucks"), and it makes first paint depend on
// a host we do not run. Vite bundles these and emits the woff2 into our own
// assets, so the app serves its own type and renders identically offline.
//
// Two weights only, matching what the surface actually uses: 400 for all body
// and metadata text, 600 for the C-code and the Q:/A: prefixes. Every extra
// weight is a file every visitor downloads to render nothing.
import "@fontsource/inter/400.css";
import "@fontsource/inter/600.css";
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
