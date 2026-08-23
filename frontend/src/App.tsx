import React from "react";
import { Navigate, Route, Routes, useLocation, useParams } from "react-router-dom";
import Header from "./components/Header";
import { DEFAULT_CASE_SLUG } from "./services/caseHeader";
import { proofMatrixPath, proofReviewTabPath } from "./utils/routePaths";
import { AuthProvider } from "./context/AuthContext";
import { CaseProvider } from "./context/CaseContext";
import AllegationsPage from "./pages/AllegationsPage";
import ContradictionsPage from "./pages/ContradictionsPage";
import AllegationDetailPage from "./pages/AllegationDetailPage";
import CaseHealthPage from "./pages/CaseHealthPage";
import CountDetailPage from "./pages/CountDetailPage";
import ProofMatrixPage from "./pages/ProofMatrixPage";
import RehearsalPage from "./pages/RehearsalPage";
import SettingsPage from "./pages/SettingsPage";
import TrialPrepDashboardPage from "./pages/TrialPrepDashboardPage";
import ScenarioDetailPage from "./pages/ScenarioDetailPage";
import PracticePage from "./pages/PracticePage";
import PracticePrintPage from "./pages/PracticePrintPage";
import PracticeQuestionReviewPage from "./pages/PracticeQuestionReviewPage";
import PracticeSessionPage from "./pages/PracticeSessionPage";
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
 * ## React Learning: why a wrapper instead of `<Navigate to="/somewhere">`
 * A bare `<Navigate to="…" replace />` drops the current URL's `?query`. The
 * Phase 2D Count tables link to `/evidence?element_id=…`; `useLocation()`
 * exposes the live location, and `Navigate`'s `to` accepts a
 * `{ pathname, search }` object, so we forward the search string verbatim.
 */
const RedirectPreservingQuery: React.FC<{ to: string }> = ({ to }) => {
  const location = useLocation();
  return <Navigate to={{ pathname: to, search: location.search }} replace />;
};

// ─── The one-release redirects (nav cleanup, Part 2) ─────────────────────────
//
// ## REMOVED IN v2.1 — every redirect in this block
//
// Each address below was real in v2.0 and is not real in v2.1. They exist so a
// bookmark Roman or Chuck already has lands on the page that replaced it rather
// than on the 404, for exactly one release. Each is pinned by a test in
// `utils/__tests__/routePaths.test.ts`; deleting one without deleting its test
// is a red build.
//
// The two `/pipeline*` redirects below them predate this task and were undated.
// They are dated with the rest here rather than left as the one undated pair in
// a block that is otherwise explicit about when it dies.
//
// `/explorer` and `/evidence` point at the proof matrix for the DEFAULT case:
// they were case-less addresses and the matrix is not, so the slug has to come
// from somewhere. `DEFAULT_CASE_SLUG` is the same constant Home and the nav
// table use — this app is single-case, and the constant is where that fact is
// already written down.
const REDIRECT_TO_MATRIX = proofMatrixPath(DEFAULT_CASE_SLUG);

/**
 * `/cases/:slug/proof-review` → the matrix, opened on its review tab.
 *
 * A component and not a bare `<Navigate to="…">` because the destination is
 * case-scoped: the slug has to be read off the route that matched before it can
 * be composed into the new address. Falling back to the default slug rather than
 * 404-ing on a missing param — the route cannot match without one, so the
 * fallback is unreachable, and an unreachable branch that lands somewhere real
 * is better than one that throws.
 *
 * REMOVED IN v2.1 with the route above it.
 */
const ProofReviewTabRedirect: React.FC = () => {
  const { slug } = useParams<{ slug: string }>();
  return <Navigate to={proofReviewTabPath(slug ?? DEFAULT_CASE_SLUG)} replace />;
};

const App: React.FC = () => {
  return (
    <AuthProvider>
      <CaseProvider>
        {/* The app shell paints the page canvas for every screen: pure white,
            v2 §2c. It was --bg-page (a grey tint) until task 1.7A — one line
            that put every screen in the product on grey. */}
        <div style={{ fontFamily: "'Inter', sans-serif", backgroundColor: "var(--bg-canvas)", minHeight: "100vh" }}>
          <Header />
          <main style={{ maxWidth: "1080px", margin: "0 auto", padding: "0 2rem" }}>
            <Routes>
              <Route path="/" element={<Home />} />
              <Route path="/allegations" element={<AllegationsPage />} />
              <Route path="/claims" element={<MotionClaimsPage />} />
              <Route path="/documents" element={<DocumentsPage />} />
              <Route path="/documents/:id" element={<DocumentWorkspaceTabs />} />
              {/* REMOVED IN v2.1 — Evidence is gone; the matrix answers the
                  same question. Query preserved: the Count tables link here
                  with `?element_id=…`. */}
              <Route path="/evidence" element={<RedirectPreservingQuery to={REDIRECT_TO_MATRIX} />} />
              <Route path="/explorer" element={<RedirectPreservingQuery to={REDIRECT_TO_MATRIX} />} />
              {/* REMOVED IN v2.1 — the Bias page is gone and nothing replaces
                  it, so Home is the honest destination. The bias FILTER half of
                  the backend stays and is used by the scenario surfaces. */}
              <Route path="/bias-explorer" element={<Navigate to="/" replace />} />
              <Route path="/damages" element={<HarmsPage />} />
              <Route path="/people" element={<People />} />
              <Route path="/people/:id" element={<PersonDetailPage />} />
              <Route path="/hearings" element={<Hearings />} />
              <Route path="/decisions" element={<Decisions />} />
              <Route path="/allegations/:id/detail" element={<AllegationDetailPage />} />
              <Route path="/cases/:slug/counts/:countId" element={<CountDetailPage />} />
              <Route path="/cases/:slug/case-health" element={<CaseHealthPage />} />
              <Route path="/cases/:slug/proof-matrix" element={<ProofMatrixPage />} />
              {/* REMOVED IN v2.1 — Proof Review is a TAB on the matrix now.
                  `ProofReviewTabRedirect` is a component rather than a bare
                  `<Navigate>` because the target is case-scoped: the slug has to
                  be read off the matched route before it can be re-composed. */}
              <Route path="/cases/:slug/proof-review" element={<ProofReviewTabRedirect />} />
              <Route path="/cases/:slug/rehearsal" element={<RehearsalPage />} />
              {/* Task 2.11 B2: the per-scenario rehearsal address. Selects within
                  the payload the page already loaded; a code nobody declared
                  ready gets the stored not-ready sentence, never a 404. */}
              <Route path="/cases/:slug/rehearsal/:code" element={<RehearsalPage />} />
              <Route path="/cases/:slug/trial-prep" element={<TrialPrepDashboardPage />} />
              <Route path="/cases/:slug/trial-prep/:scenarioId" element={<ScenarioDetailPage />} />
              {/* PRACTICE v0: Marie's drill for one scenario. A longer path than
                  the scenario page's, so React Router's specificity ranking
                  cannot confuse the two — `practice` is a literal segment where
                  that route has `:scenarioId`, and this one has an id after it. */}
              <Route path="/cases/:slug/trial-prep/practice/:scenarioId" element={<PracticePage />} />
              {/* PRACTICE flow v1 Section B: the SITTING's own address, so the
                  browser's Back button and a mid-session reload both work. One
                  segment longer again, and `session` is a literal where the
                  parent route ends — neither can shadow the other. */}
              <Route
                path="/cases/:slug/trial-prep/practice/:scenarioId/session/:sessionId"
                element={<PracticeSessionPage />}
              />
              {/* PRACTICE v1 Part B: one question's review page — every attempt
                  at it, newest first. `question` is a literal where the sibling
                  has `session`, so neither can shadow the other. */}
              <Route
                path="/cases/:slug/trial-prep/practice/:scenarioId/question/:questionId"
                element={<PracticeQuestionReviewPage />}
              />
              {/* Chuck's review sheets. A page of its own rather than a print
                  stylesheet on the practice page: that page's deck list is
                  conditionally rendered behind a fold and filtered by the
                  *Who's asking?* selector, so its DOM is not the whole deck. */}
              <Route
                path="/cases/:slug/trial-prep/practice/:scenarioId/print"
                element={<PracticePrintPage />}
              />
              <Route path="/contradictions" element={<ContradictionsPage />} />
              <Route path="/graph" element={<GraphPage />} />
              <Route path="/queries" element={<QueriesPage />} />
              <Route path="/search" element={<SearchPage />} />
              <Route path="/ask" element={<AskPage />} />
              <Route path="/timeline" element={<TimelinePage />} />
              {/* Admin: five ADDRESSES where there were nine tabs in component
                  state. A tab nobody can bookmark, return to with Back, or link
                  to is a place that does not exist as far as the browser is
                  concerned. */}
              <Route path="/admin" element={<Admin group="overview" />} />
              <Route path="/admin/prompts" element={<Admin group="prompts" />} />
              <Route path="/admin/data" element={<Admin group="data" />} />
              <Route path="/admin/logs" element={<Admin group="logs" />} />
              <Route path="/admin/settings" element={<SettingsPage />} />
              {/* REMOVED IN v2.1 — Settings moved under Admin. Roman's bookmark. */}
              <Route path="/settings" element={<Navigate to="/admin/settings" replace />} />
              {/* REMOVED IN v2.1 — predates this task, dated with the rest
                  rather than left as the one undated pair in the block. */}
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
