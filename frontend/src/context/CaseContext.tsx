import React, { createContext, useContext, useEffect, useState } from "react";
import { CaseResponse, getCase } from "../services/case";

// Context value type
type CaseContextValue = {
  caseData: CaseResponse | null;
  loading: boolean;
  error: string | null;
};

// Create context with undefined default (will throw if used outside provider)
const CaseContext = createContext<CaseContextValue | undefined>(undefined);

// Provider component that fetches case data on mount
export const CaseProvider: React.FC<{ children: React.ReactNode }> = ({
  children,
}) => {
  const [caseData, setCaseData] = useState<CaseResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;

    async function fetchCase() {
      try {
        const data = await getCase();
        if (!cancelled) {
          setCaseData(data);
          setLoading(false);
        }
      } catch (err) {
        if (!cancelled) {
          const message =
            err instanceof Error ? err.message : "Failed to load case data";
          setError(message);
          setLoading(false);
        }
      }
    }

    fetchCase();

    // Cleanup function to prevent state updates after unmount
    return () => {
      cancelled = true;
    };
  }, []);

  return (
    <CaseContext.Provider value={{ caseData, loading, error }}>
      {children}
    </CaseContext.Provider>
  );
};

// Hook to access case data - throws if used outside CaseProvider
export function useCase(): CaseContextValue {
  const context = useContext(CaseContext);
  if (context === undefined) {
    throw new Error("useCase must be used within a CaseProvider");
  }
  return context;
}
