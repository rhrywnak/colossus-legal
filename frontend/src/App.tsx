import React from "react";
import { Navigate, Route, Routes, useLocation } from "react-router-dom";
import Header from "./components/Header";
import { AuthProvider } from "./context/AuthContext";
import { CaseProvider } from "./context/CaseContext";
import AllegationsPage from "./pages/AllegationsPage";
import ContradictionsPage from "./pages/ContradictionsPage";
import AllegationDetailPage from "./pages/AllegationDetailPage";
import CaseHealthPage from "./pages/CaseHealthPage";
import CountDetailPage from "./pages/CountDetailPage";
import ProofMatrixPage from "./pages/ProofMatrixPage";
import ProofReviewPage from "./pages/ProofReviewPage";
import TrialPrepDashboardPage from "./pages/TrialPrepDashboardPage";
import ScenarioDetailPage from "./pages/ScenarioDetailPage";
import BiasExplorer from "./pages/BiasExplorer";
import EvidenceExplorerPage from "./pages/EvidenceExplorerPage";
import GraphPage from "./pages/GraphPage";
import QueriesPage from "./pages/QueriesPage";
import AskPage from "./pages/AskPage";
import SearchPage from "./pages/SearchPage";
import MotionClaimsPage from "./pages/MotionClaimsPage";
import Decisions from "./pages/Decisions";
import DocumentsPage from "./pages/DocumentsPage";
import DocumentWorkspaceTabs from "./pages/DocumentWorkspaceTabs";
import HarmsPage from "./pages/HarmsPage";
import Hearings from "./pages/Hearings";
import Home from "./pages/Home";
import NotFoundPage from "./pages/NotFoundPage";
import Admin from "./pages/Admin";
import People from "./pages/People";
import PersonDetailPage from "./pages/PersonDetailPage";
import TimelinePage from "./pages/TimelinePage";

/**
 * Redirect that preserves the query string while changing the path.
 *
 * ## React Learning: why a wrapper instead of `<Navigate to="/explorer">`
 * A bare `<Navigate to="/explorer" replace />` drops the current URL's `?query`.
 * The Phase 2D Count tables link to `/evidence?element_id=…`, and `/evidence`
 * is an alias that redirects to the real Evidence tab at `/explorer` — so a bare
 * redirect would silently lose `element_id` on the hop. `useLocation()` exposes
 * the live location, and `Navigate`'s `to` accepts a `{ pathname, search }`
 * object, so we forward the search string verbatim to the target path.
 */
const RedirectPreservingQuery: React.FC<{ to: string }> = ({ to }) => {
  const location = useLocation();
  return <Navigate to={{ pathname: to, search: location.search }} replace />;
};

const App: React.FC = () => {
  return (
    <AuthProvider>
      <CaseProvider>
        <div style={{ fontFamily: "'Inter', sans-serif", backgroundColor: "var(--bg-page)", minHeight: "100vh" }}>
          <Header />
          <main style={{ maxWidth: "1080px", margin: "0 auto", padding: "0 2rem" }}>
            <Routes>
              <Route path="/" element={<Home />} />
              <Route path="/allegations" element={<AllegationsPage />} />
              <Route path="/claims" element={<MotionClaimsPage />} />
              <Route path="/documents" element={<DocumentsPage />} />
              <Route path="/documents/:id" element={<DocumentWorkspaceTabs />} />
              <Route path="/evidence" element={<RedirectPreservingQuery to="/explorer" />} />
              <Route path="/damages" element={<HarmsPage />} />
              <Route path="/people" element={<People />} />
              <Route path="/people/:id" element={<PersonDetailPage />} />
              <Route path="/hearings" element={<Hearings />} />
              <Route path="/decisions" element={<Decisions />} />
              <Route path="/allegations/:id/detail" element={<AllegationDetailPage />} />
              <Route path="/cases/:slug/counts/:countId" element={<CountDetailPage />} />
              <Route path="/cases/:slug/case-health" element={<CaseHealthPage />} />
              <Route path="/cases/:slug/proof-matrix" element={<ProofMatrixPage />} />
              <Route path="/cases/:slug/proof-review" element={<ProofReviewPage />} />
              <Route path="/cases/:slug/trial-prep" element={<TrialPrepDashboardPage />} />
              <Route path="/cases/:slug/trial-prep/:scenarioId" element={<ScenarioDetailPage />} />
              <Route path="/contradictions" element={<ContradictionsPage />} />
              <Route path="/explorer" element={<EvidenceExplorerPage />} />
              <Route path="/bias-explorer" element={<BiasExplorer />} />
              <Route path="/graph" element={<GraphPage />} />
              <Route path="/queries" element={<QueriesPage />} />
              <Route path="/search" element={<SearchPage />} />
              <Route path="/ask" element={<AskPage />} />
              <Route path="/timeline" element={<TimelinePage />} />
              <Route path="/admin" element={<Admin />} />
              <Route path="/pipeline" element={<Navigate to="/documents" replace />} />
              <Route path="/pipeline/:id" element={<Navigate to="/documents" replace />} />
              {/* Catch-all, and it must stay LAST: React Router v6 ranks routes by
                  specificity rather than declaration order, so `*` cannot shadow a
                  real route — but keeping it last is what makes that obvious to a
                  reader adding the next one. Without it an unmatched URL rendered
                  the Header over an empty <main>, which reads as a page that
                  failed to load rather than one that does not exist (the
                  /analysis and /decomposition retirement is what exposed it). */}
              <Route path="*" element={<NotFoundPage />} />
            </Routes>
          </main>
        </div>
      </CaseProvider>
    </AuthProvider>
  );
};

export default App;
