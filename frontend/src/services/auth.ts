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
 * Drop-in replacement for fetch() that adds `credentials: 'include'`.
 * Same signature as window.fetch — all service files can swap with no
 * other changes.
 */
export async function authFetch(
    url: string,
    options?: RequestInit,
): Promise<Response> {
    return fetch(url, {
        ...options,
        credentials: "include",
    });
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
