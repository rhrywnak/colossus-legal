import React, { useEffect, useState } from "react";
import { getStatus, StatusResponse } from "../services/api";

const Home: React.FC = () => {
  const [status, setStatus] = useState<StatusResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let active = true;

    const fetchStatus = async () => {
      try {
        const result = await getStatus();
        if (!active) return;
        setStatus(result);
        setError(null);
      } catch {
        if (!active) return;
        setStatus(null);
        setError("Backend unreachable");
      } finally {
        if (active) {
          setLoading(false);
        }
      }
    };

    fetchStatus();

    return () => {
      active = false;
    };
  }, []);

  const statusMessage = () => {
    if (loading) return "Checking backend status...";
    if (error) return "Backend unreachable";
    if (status)
      return `Backend OK – ${status.app} v${status.version} (status: ${status.status})`;
    return null;
  };

  return (
    <div>
      <div
        style={{
          padding: "0.75rem 1rem",
          border: "1px solid #ddd",
          borderRadius: "6px",
          marginBottom: "1rem",
          backgroundColor: "#f8f9fa",
        }}
      >
        {statusMessage()}
      </div>
      <h1>Colossus-Legal</h1>
      <p>
        Skeleton is in place. Backend availability is checked via{" "}
        <code>/api/status</code> from <code>VITE_API_URL</code>. This UI will
        eventually evolve into:
      </p>
      <ul>
        <li>Document library & upload</li>
        <li>AI suggestion review queue</li>
        <li>Graph and timeline views</li>
        <li>Reporting and exports</li>
      </ul>
    </div>
  );
};

export default Home;
