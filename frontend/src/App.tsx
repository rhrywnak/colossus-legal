import React from "react";
import { Route, Routes } from "react-router-dom";
import Header from "./components/Header";
import { AuthProvider } from "./context/AuthContext";
import { CaseProvider } from "./context/CaseContext";
import AllegationsPage from "./pages/AllegationsPage";
import AnalysisPage from "./pages/AnalysisPage";
import ContradictionsPage from "./pages/ContradictionsPage";
import DecompositionPage from "./pages/DecompositionPage";
import AllegationDetailPage from "./pages/AllegationDetailPage";
import EvidenceExplorerPage from "./pages/EvidenceExplorerPage";
import GraphPage from "./pages/GraphPage";
import QueriesPage from "./pages/QueriesPage";
import AskPage from "./pages/AskPage";
import SearchPage from "./pages/SearchPage";
import MotionClaimsPage from "./pages/MotionClaimsPage";
import Decisions from "./pages/Decisions";
import DocumentDetailPage from "./pages/DocumentDetailPage";
import DocumentsPage from "./pages/DocumentsPage";
import EvidencePage from "./pages/EvidencePage";
import HarmsPage from "./pages/HarmsPage";
import Hearings from "./pages/Hearings";
import Home from "./pages/Home";
import People from "./pages/People";
import PersonDetailPage from "./pages/PersonDetailPage";

const App: React.FC = () => {
  return (
    <AuthProvider>
      <CaseProvider>
        <div style={{ fontFamily: "'DM Sans', sans-serif", backgroundColor: "#f0f2f5", minHeight: "100vh" }}>
          <Header />
          <main style={{ maxWidth: "1080px", margin: "0 auto", padding: "0 2rem" }}>
            <Routes>
              <Route path="/" element={<Home />} />
              <Route path="/analysis" element={<AnalysisPage />} />
              <Route path="/allegations" element={<AllegationsPage />} />
              <Route path="/claims" element={<MotionClaimsPage />} />
              <Route path="/documents" element={<DocumentsPage />} />
              <Route path="/documents/:id" element={<DocumentDetailPage />} />
              <Route path="/evidence" element={<EvidencePage />} />
              <Route path="/damages" element={<HarmsPage />} />
              <Route path="/people" element={<People />} />
              <Route path="/people/:id" element={<PersonDetailPage />} />
              <Route path="/hearings" element={<Hearings />} />
              <Route path="/decisions" element={<Decisions />} />
              <Route path="/decomposition" element={<DecompositionPage />} />
              <Route path="/allegations/:id/detail" element={<AllegationDetailPage />} />
              <Route path="/contradictions" element={<ContradictionsPage />} />
              <Route path="/explorer" element={<EvidenceExplorerPage />} />
              <Route path="/graph" element={<GraphPage />} />
              <Route path="/queries" element={<QueriesPage />} />
              <Route path="/search" element={<SearchPage />} />
              <Route path="/ask" element={<AskPage />} />
            </Routes>
          </main>
        </div>
      </CaseProvider>
    </AuthProvider>
  );
};

export default App;
