import React, { useEffect, useState } from "react";
import { Claim, getClaims } from "../services/claims";

const ClaimsPage: React.FC = () => {
  const [claims, setClaims] = useState<Claim[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let active = true;

    const loadClaims = async () => {
      try {
        const data = await getClaims();
        if (!active) return;
        setClaims(data);
        setError(null);
      } catch (err) {
        if (!active) return;
        const message = err instanceof Error ? err.message : "Unknown error";
        setError(message);
      } finally {
        if (active) {
          setLoading(false);
        }
      }
    };

    loadClaims();

    return () => {
      active = false;
    };
  }, []);

  if (loading) {
    return <div>Loading claims...</div>;
  }

  if (error) {
    return <div role="alert">Error loading claims: {error}</div>;
  }

  if (claims.length === 0) {
    return <div>No claims yet.</div>;
  }

  return (
    <div>
      <h2>Claims</h2>
      <p>Showing claims from backend.</p>
      <ul style={{ paddingLeft: "1.25rem" }}>
        {claims.map((claim) => (
          <li key={claim.id} style={{ marginBottom: "0.5rem" }}>
            <strong>{claim.title}</strong> <em>({claim.status})</em>
            {claim.description ? (
              <div style={{ color: "#555", marginTop: "0.15rem" }}>
                {claim.description}
              </div>
            ) : null}
          </li>
        ))}
      </ul>
    </div>
  );
};

export default ClaimsPage;
