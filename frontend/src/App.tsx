import React from "react";
import { Link, Route, Routes } from "react-router-dom";
import AllegationsPage from "./pages/AllegationsPage";
import ClaimsPage from "./pages/ClaimsPage";
import Decisions from "./pages/Decisions";
import DocumentDetailPage from "./pages/DocumentDetailPage";
import DocumentsPage from "./pages/DocumentsPage";
import EvidencePage from "./pages/EvidencePage";
import HarmsPage from "./pages/HarmsPage";
import Hearings from "./pages/Hearings";
import Home from "./pages/Home";
import People from "./pages/People";

const navLinks = [
  { to: "/", label: "Home" },
  { to: "/allegations", label: "Allegations" },
  { to: "/documents", label: "Documents" },
  { to: "/evidence", label: "Evidence" },
  { to: "/damages", label: "Damages" },
  { to: "/people", label: "People" },
  { to: "/hearings", label: "Hearings" },
  { to: "/decisions", label: "Decisions" },
];

const App: React.FC = () => {
  return (
    <div style={{ padding: "1.5rem", fontFamily: "Inter, system-ui, sans-serif" }}>
      <header style={{ marginBottom: "1.5rem" }}>
        <h1 style={{ margin: "0 0 0.5rem 0" }}>Colossus-Legal</h1>
        <nav style={{ display: "flex", gap: "0.75rem", flexWrap: "wrap" }}>
          {navLinks.map((link) => (
            <Link
              key={link.to}
              to={link.to}
              style={{
                padding: "0.35rem 0.6rem",
                border: "1px solid #e0e0e0",
                borderRadius: "6px",
                textDecoration: "none",
                color: "#333",
                background: "#f5f5f5",
              }}
            >
              {link.label}
            </Link>
          ))}
        </nav>
      </header>

      <main>
        <Routes>
          <Route path="/" element={<Home />} />
          <Route path="/allegations" element={<AllegationsPage />} />
          <Route path="/claims" element={<ClaimsPage />} />
          <Route path="/documents" element={<DocumentsPage />} />
          <Route path="/documents/:id" element={<DocumentDetailPage />} />
          <Route path="/evidence" element={<EvidencePage />} />
          <Route path="/damages" element={<HarmsPage />} />
          <Route path="/people" element={<People />} />
          <Route path="/hearings" element={<Hearings />} />
          <Route path="/decisions" element={<Decisions />} />
        </Routes>
      </main>
    </div>
  );
};

export default App;
