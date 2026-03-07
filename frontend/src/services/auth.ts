import { API_BASE_URL } from "./api";

// =============================================================================
// Auth Service — Credentialed Fetch Wrapper + User Info
// =============================================================================
//
// WHAT THIS DOES:
// Every fetch() call in the app needs to send credentials (cookies) so the
// backend knows who the user is. Instead of adding `credentials: 'include'`
// to every single fetch call, we wrap fetch() once here and every service
// file imports authFetch instead of using raw fetch.
//
// WHY credentials: 'include'?
// When the frontend (localhost:5473) talks to the backend (localhost:3403),
// that's a cross-origin request. Browsers do NOT send cookies on cross-origin
// requests by default — you must explicitly opt in with `credentials: 'include'`.
// On the backend side, CORS must also have `allow_credentials(true)`.
// =============================================================================

// ─── Types ──────────────────────────────────────────────────────────────────

export type AuthPermissions = {
    can_read: boolean;
    can_edit: boolean;
    can_use_ai: boolean;
    is_admin: boolean;
};

export type AuthUser = {
    username: string;
    display_name: string;
    email: string;
    groups: string[];
    permissions: AuthPermissions;
};

// ─── Credentialed fetch wrapper ─────────────────────────────────────────────

/**
 * Drop-in replacement for fetch() that adds credentials and a timeout.
 * Default timeout is 30s; callers can override via options.signal or
 * by passing a custom timeout.
 */
export async function authFetch(
    url: string,
    options?: RequestInit & { timeoutMs?: number },
): Promise<Response> {
    const { timeoutMs = 30000, ...fetchOptions } = options ?? {};

    // If caller already provided a signal, respect it; otherwise create a timeout
    if (fetchOptions.signal) {
        return fetch(url, {
            ...fetchOptions,
            credentials: "include",
        });
    }

    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), timeoutMs);
    try {
        return await fetch(url, {
            ...fetchOptions,
            credentials: "include",
            signal: controller.signal,
        });
    } finally {
        clearTimeout(timeoutId);
    }
}

// ─── Get current user ───────────────────────────────────────────────────────

/**
 * Calls GET /api/me to get the current authenticated user.
 * Returns null if the user is anonymous or the request fails.
 */
export async function getCurrentUser(): Promise<AuthUser | null> {
    try {
        const response = await authFetch(`${API_BASE_URL}/api/me`);
        if (!response.ok) return null;

        const data = await response.json();

        // Backend returns { anonymous: true, ... } for unauthenticated users
        if (data.anonymous) return null;

        return data as AuthUser;
    } catch {
        return null;
    }
}

// ─── Logout ─────────────────────────────────────────────────────────────────

/**
 * Logout via the backend /api/logout endpoint.
 *
 * Navigates (not fetch!) to the backend, which expires the Authentik proxy
 * cookie via Set-Cookie and 302-redirects to the OIDC end-session endpoint.
 *
 * Uses .replace() so the logout page replaces the current history entry,
 * preventing "back-button" identity ghosting.
 */
export function logout(): void {
    const config = (window as any).__COLOSSUS_CONFIG__ || {};
    const apiUrl = config.apiUrl || "";
    // Navigate (not fetch!) to the backend logout endpoint.
    // It expires the proxy cookie via Set-Cookie and 302s to Authentik end-session.
    window.location.replace(`${apiUrl}/api/logout`);
}
