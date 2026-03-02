import React, { createContext, useContext, useEffect, useState } from "react";
import { AuthUser, getCurrentUser } from "../services/auth";

// =============================================================================
// AuthContext — Provides current-user info to the entire component tree
// =============================================================================
//
// PATTERN: Follows the same createContext → Provider → useHook pattern as
// CaseContext.tsx. The Provider fetches once on mount, and every component
// that needs user info calls useAuth().
//
// WHY A CONTEXT?
// Without a context, every component that needs the user would have to call
// getCurrentUser() independently, each making its own HTTP request. With a
// context, we fetch once and share the result via React's context mechanism.
// =============================================================================

type AuthContextValue = {
    /** The authenticated user, or null if anonymous / not yet loaded */
    user: AuthUser | null;
    /** True while the initial /api/me request is in flight */
    loading: boolean;
    /** Convenience: true when user is not null */
    isAuthenticated: boolean;
};

const AuthContext = createContext<AuthContextValue | undefined>(undefined);

// ─── Provider ───────────────────────────────────────────────────────────────

export const AuthProvider: React.FC<{ children: React.ReactNode }> = ({
    children,
}) => {
    const [user, setUser] = useState<AuthUser | null>(null);
    const [loading, setLoading] = useState(true);

    useEffect(() => {
        let cancelled = false;

        async function fetchUser() {
            const u = await getCurrentUser();
            if (!cancelled) {
                setUser(u);
                setLoading(false);
            }
        }

        fetchUser();

        // Cleanup: prevent state updates if the component unmounts before
        // the fetch completes (same pattern as CaseContext).
        return () => {
            cancelled = true;
        };
    }, []);

    return (
        <AuthContext.Provider
            value={{ user, loading, isAuthenticated: user !== null }}
        >
            {children}
        </AuthContext.Provider>
    );
};

// ─── Hook ───────────────────────────────────────────────────────────────────

/**
 * Access the current auth state. Must be used inside <AuthProvider>.
 */
export function useAuth(): AuthContextValue {
    const context = useContext(AuthContext);
    if (context === undefined) {
        throw new Error("useAuth must be used within an AuthProvider");
    }
    return context;
}
