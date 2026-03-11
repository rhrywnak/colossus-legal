import React, { useEffect, useState } from "react";
import { getContradictions, ContradictionDto } from "../services/contradictions";
import ImpeachmentCard from "../components/ImpeachmentCard";

const ContradictionsPage: React.FC = () => {
  const [contradictions, setContradictions] = useState<ContradictionDto[]>([]);
  const [total, setTotal] = useState(0);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let active = true;
    getContradictions()
      .then((result) => { if (active) { setContradictions(result.contradictions); setTotal(result.total); setError(null); } })
      .catch(() => { if (active) { setContradictions([]); setError("Failed to load impeachment evidence"); } })
      .finally(() => { if (active) setLoading(false); });
    return () => { active = false; };
  }, []);

  if (loading) {
    return <div style={{ padding: "2rem", textAlign: "center", color: "#6b7280" }}>Loading impeachment evidence...</div>;
  }
  if (error) {
    return (
      <div style={{ padding: "1rem", backgroundColor: "#fef2f2", border: "1px solid #fecaca", borderRadius: "6px", color: "#dc2626" }}>
        {error}
      </div>
    );
  }

  return (
    <div>
      <h1 style={{ marginBottom: "0.5rem" }}>Impeachment Evidence ({total})</h1>
      <p style={{ color: "#6b7280", marginBottom: "1.5rem", fontSize: "0.9rem" }}>
        Prior statements contradicted by later admissions under oath — key material for cross-examination.
      </p>

      {contradictions.length === 0 ? (
        <div style={{ color: "#6b7280", padding: "1rem" }}>No impeachment evidence found.</div>
      ) : (
        <div style={{ display: "flex", flexDirection: "column", gap: "0.25rem" }}>
          {contradictions.map((c, i) => (
            <ImpeachmentCard key={i} contradiction={c} />
          ))}
        </div>
      )}
    </div>
  );
};

export default ContradictionsPage;
