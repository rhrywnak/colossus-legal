// =============================================================================
// API Configuration — Runtime Config with Local Dev Fallback
// =============================================================================
//
// RUST PARALLEL: This is similar to how your Rust backend reads config from
// environment variables at startup (AppConfig::from_env). The frontend equivalent
// is reading from window.__COLOSSUS_CONFIG__ which gets injected at container
// startup, or falling back to VITE_API_URL for local development.
//
// WHY RUNTIME CONFIG?
// Vite normally bakes VITE_API_URL into the JS bundle at build time. That means
// you'd need separate docker builds for DEV and PROD. With runtime config,
// ONE image works everywhere — the container entrypoint writes config.js with
// the correct API URL before nginx starts serving.
//
// RESOLUTION ORDER:
// 1. window.__COLOSSUS_CONFIG__.apiUrl  (container deployment)
// 2. import.meta.env.VITE_API_URL       (npm run dev)
// 3. "http://localhost:3403"             (bare fallback)
// =============================================================================

export type StatusResponse = {
    app: string;
    version: string;
    status: string;
};

// TypeScript: Extend the Window interface to include our runtime config.
// The 'declare global' block tells TypeScript "this property might exist on window"
// without actually creating it — the container entrypoint creates it at runtime.
//
// authLogoutUrl is optional so that older deployments that haven't updated their
// docker-entrypoint.sh yet still satisfy the type — Header.tsx falls back to a
// hardcoded default if it's missing (same pattern as Rust's Option<T>).
declare global {
    interface Window {
        __COLOSSUS_CONFIG__?: {
            apiUrl: string;
            authLogoutUrl?: string;
            environment?: string;
            version?: string;
        };
    }
}

// Single source of truth for the API base URL.
// Every service file imports this — no other files need to change.
// The typeof guard handles test environments (Vitest/Node) where window is undefined.
export const API_BASE_URL: string =
    (typeof window !== "undefined" && window.__COLOSSUS_CONFIG__?.apiUrl)
    || import.meta.env.VITE_API_URL
    || "http://localhost:3403";

// NOTE: Circular import — auth.ts imports API_BASE_URL from this file, and
// this file imports authFetch from auth.ts. This works fine in ESM because
// both values are only used inside function bodies, not at module-load time.
// By the time any function runs, both modules are fully initialized.
import { authFetch } from "./auth";

export async function getStatus(): Promise<StatusResponse> {
    const response = await authFetch(`${API_BASE_URL}/api/status`);

    if (!response.ok) {
        throw new Error(`Status request failed with ${response.status}`);
    }

    let data: unknown;
    try {
        data = await response.json();
    } catch (error) {
        throw new Error("Failed to parse status response");
    }

    const parsed = data as Partial<StatusResponse>;
    if (!parsed.app || !parsed.version || !parsed.status) {
        throw new Error("Invalid status response shape");
    }

    return {
        app: parsed.app,
        version: parsed.version,
        status: parsed.status,
    };
}
