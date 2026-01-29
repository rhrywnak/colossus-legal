import React from "react";
import { Route, Routes } from "react-router-dom";
import Header from "./components/Header";
import { CaseProvider } from "./context/CaseContext";
import AllegationsPage from "./pages/AllegationsPage";
import ContradictionsPage from "./pages/ContradictionsPage";
import EvidenceExplorerPage from "./pages/EvidenceExplorerPage";
import GraphPage from "./pages/GraphPage";
import MotionClaimsPage from "./pages/MotionClaimsPage";
import Decisions from "./pages/Decisions";
import DocumentDetailPage from "./pages/DocumentDetailPage";
import DocumentsPage from "./pages/DocumentsPage";
import EvidencePage from "./pages/EvidencePage";
import HarmsPage from "./pages/HarmsPage";
import Hearings from "./pages/Hearings";
import Home from "./pages/Home";
import People from "./pages/People";

const App: React.FC = () => {
  return (
    <CaseProvider>
      <div style={{ fontFamily: "Inter, system-ui, sans-serif" }}>
        <Header />
        <main style={{ padding: "1.5rem" }}>
          <Routes>
            <Route path="/" element={<Home />} />
            <Route path="/allegations" element={<AllegationsPage />} />
            <Route path="/claims" element={<MotionClaimsPage />} />
            <Route path="/documents" element={<DocumentsPage />} />
            <Route path="/documents/:id" element={<DocumentDetailPage />} />
            <Route path="/evidence" element={<EvidencePage />} />
            <Route path="/damages" element={<HarmsPage />} />
            <Route path="/people" element={<People />} />
            <Route path="/hearings" element={<Hearings />} />
            <Route path="/decisions" element={<Decisions />} />
            <Route path="/contradictions" element={<ContradictionsPage />} />
            <Route path="/explorer" element={<EvidenceExplorerPage />} />
            <Route path="/graph" element={<GraphPage />} />
          </Routes>
        </main>
      </div>
    </CaseProvider>
  );
};

export default App;
