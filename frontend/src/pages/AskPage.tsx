import React, { useEffect, useRef, useState } from "react";
import { useSearchParams } from "react-router-dom";
import { askTheCase, AskResponse } from "../services/ask";
import {
  getQAHistory, getQAEntry, rateQAEntry, deleteQAEntry, mapEntryToResponse,
  QAEntrySummary, QAEntryFull,
} from "../services/qa";
import AnswerDisplay from "../components/AnswerDisplay";
import MetricsBar from "../components/MetricsBar";
import RetrievalDetailsPanel from "../components/RetrievalDetailsPanel";
import { HistoryCard } from "../components/HistoryCard";
import { pageText } from "../config/pageText";

const AskPage: React.FC = () => {
  const [searchParams] = useSearchParams();
  const initialQuestion = searchParams.get("q") || "";

  const [question, setQuestion] = useState(initialQuestion);
  const [response, setResponse] = useState<AskResponse | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [phase, setPhase] = useState("");
  const [activeTab, setActiveTab] = useState<"ask" | "history">("ask");
  const [history, setHistory] = useState<QAEntrySummary[]>([]);
  const [historyLoading, setHistoryLoading] = useState(false);
  const [viewingHistoryEntry, setViewingHistoryEntry] = useState<QAEntryFull | null>(null);
  const [parentQaId, setParentQaId] = useState<string | null>(null);

  const loadHistory = async () => {
    setHistoryLoading(true);
    try {
      const data = await getQAHistory("case", "awad-v-cfs-2011", 20);
      setHistory(data);
    } catch (e) {
      console.error("Failed to load history", e);
    } finally {
      setHistoryLoading(false);
    }
  };

  const handleTabSwitch = (tab: "ask" | "history") => {
    setActiveTab(tab);
    if (tab === "history") {
      loadHistory();
    } else {
      handleClear();
    }
  };

  const autoSubmitted = useRef(false);

  const submitQuestion = async (q: string) => {
    if (!q.trim() || loading) return;
    setLoading(true);
    setError(null);
    setResponse(null);
    setViewingHistoryEntry(null);
    setPhase("Embedding question...");
    const t1 = setTimeout(() => setPhase("Searching evidence..."), 800);
    const t2 = setTimeout(() => setPhase("Expanding graph context..."), 2000);
    const t3 = setTimeout(() => setPhase("Synthesizing answer..."), 3500);

    try {
      const result = await askTheCase(q.trim(), parentQaId);
      setResponse(result);
      setParentQaId(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Unknown error");
    } finally {
      clearTimeout(t1);
      clearTimeout(t2);
      clearTimeout(t3);
      setLoading(false);
      setPhase("");
    }
  };

  const handleSubmit = (e: React.FormEvent) => { e.preventDefault(); submitQuestion(question); };

  useEffect(() => {
    if (initialQuestion && !autoSubmitted.current) {
      autoSubmitted.current = true;
      submitQuestion(initialQuestion);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [initialQuestion]);

  const handleClear = () => {
    setQuestion("");
    setResponse(null);
    setError(null);
    setViewingHistoryEntry(null);
    setParentQaId(null);
  };

  // Load a full QA entry from history and display its answer
  const handleHistoryClick = async (entryId: string) => {
    setHistoryLoading(true);
    try {
      const full = await getQAEntry(entryId);
      setViewingHistoryEntry(full);
      setQuestion(full.question);
      // Map QAEntryFull to AskResponse so AnswerDisplay can render it
      setResponse(mapEntryToResponse(full));
      setError(null);
      setActiveTab("ask");
    } catch (e) {
      console.error("Failed to load history entry", e);
    } finally {
      setHistoryLoading(false);
    }
  };

  const handleDelete = async (entryId: string) => {
    const success = await deleteQAEntry(entryId);
    if (success) {
      setHistory((prev) => prev.filter((e) => e.id !== entryId));
    }
  };

  const handleFollowUp = () => {
    // The current response's qa_id becomes the parent for the follow-up.
    // For history entries we have the id directly; for live answers we use
    // the response's qa_id if available, falling back to the history entry id.
    const currentId = viewingHistoryEntry?.id ?? null;
    setParentQaId(currentId);
    setQuestion("");
  };

  // Whether an answer is currently displayed (live or from history)
  const answerDisplayed = response && !loading;
  // The current qa_id for follow-up — history entries always have an id
  const canFollowUp = answerDisplayed && viewingHistoryEntry != null;

  return (
    <div style={{ maxWidth: "900px", margin: "0 auto", paddingBottom: "2rem" }}>
      {/* Header */}
      <div style={{ marginBottom: "1.5rem" }}>
        <h1 style={{ margin: 0, fontSize: "1.75rem" }}>{pageText.ask.title}</h1>
        <p style={{ margin: "0.25rem 0 0", color: "#6b7280", fontSize: "0.9rem" }}>
          {pageText.ask.subtitle}
        </p>
      </div>

      {/* Tab toggle — right-aligned, matching submit button color */}
      <div style={{ display: "flex", justifyContent: "flex-end", marginBottom: "1rem" }}>
        <button
          onClick={() => handleTabSwitch(activeTab === "ask" ? "history" : "ask")}
          style={{
            padding: "0.5rem 1.25rem", borderRadius: "7px", cursor: "pointer",
            border: "none", backgroundColor: "#2563eb",
            color: "#ffffff", fontWeight: 500, fontSize: "0.84rem",
          }}
        >
          {activeTab === "ask" ? pageText.ask.historyTab : pageText.ask.title}
        </button>
      </div>

      {/* Ask tab */}
      {activeTab === "ask" && (<>
      {/* Follow-up indicator */}
      {parentQaId && (
        <div style={{ fontSize: "0.8rem", color: "#2563eb", marginBottom: "0.5rem",
          display: "flex", alignItems: "center", gap: "0.5rem" }}>
          <span>Following up on previous question</span>
          <button onClick={() => setParentQaId(null)} style={{
            background: "none", border: "none", color: "#ef4444",
            cursor: "pointer", fontSize: "0.8rem" }}>✕ Cancel</button>
        </div>
      )}

      {/* Question form */}
      <form onSubmit={handleSubmit} style={{ marginBottom: "1.5rem" }}>
        {/* Textarea with overlay Ask button */}
        <div style={{ position: "relative" }}>
          <textarea
            value={question}
            onChange={(e) => setQuestion(e.target.value)}
            placeholder={pageText.ask.placeholder}
            rows={3}
            style={{
              width: "100%", padding: "0.75rem 3.5rem 0.75rem 1rem", border: "1px solid #d1d5db",
              borderRadius: "8px", fontSize: "1rem", fontFamily: "inherit",
              resize: "vertical", outline: "none", boxSizing: "border-box",
            }}
          />
          <button
            type="submit"
            disabled={loading || !question.trim()}
            style={{
              position: "absolute", bottom: "10px", right: "10px",
              width: "40px", height: "40px", borderRadius: "7px",
              backgroundColor: loading || !question.trim() ? "#93c5fd" : "#2563eb",
              color: "#fff", border: "none", cursor: loading ? "wait" : "pointer",
              display: "flex", alignItems: "center", justifyContent: "center",
              fontSize: "1.4rem", fontWeight: 700, lineHeight: 1,
            }}
            title={loading ? "Thinking..." : "Ask"}
          >
            {loading ? "\u2026" : "\u2191"}
          </button>
        </div>
        {response && (
          <div style={{ marginTop: "0.5rem" }}>
            <button
              type="button"
              onClick={handleClear}
              style={{
                padding: "0.5rem 1.25rem", backgroundColor: "#f3f4f6",
                color: "#374151", border: "1px solid #d1d5db", borderRadius: "6px",
                fontSize: "0.9rem", cursor: "pointer",
              }}
            >
              Ask a New Question
            </button>
          </div>
        )}
      </form>

      {/* Loading phase indicator */}
      {loading && phase && (
        <div style={{
          padding: "1.5rem", textAlign: "center", color: "#2563eb",
          backgroundColor: "#eff6ff", borderRadius: "8px", marginBottom: "1rem",
        }}>
          <div style={{ fontSize: "1.1rem", fontWeight: 500, marginBottom: "0.25rem" }}>
            {phase}
          </div>
          <div style={{ fontSize: "0.8rem", color: "#6b7280" }}>
            This usually takes 5-15 seconds
          </div>
        </div>
      )}

      {/* Error */}
      {error && (
        <div style={{
          padding: "1rem", backgroundColor: "#fef2f2", border: "1px solid #fecaca",
          borderRadius: "8px", color: "#dc2626", marginBottom: "1rem",
        }}>
          {error}
        </div>
      )}

      {/* Metrics bar — shown after response, before the answer */}
      {response && !loading && <MetricsBar response={response} />}
      {response && !loading && <RetrievalDetailsPanel response={response} />}

      {/* Historical answer header */}
      {viewingHistoryEntry && response && (
        <div style={{ fontSize: "0.8rem", color: "#6b7280", marginBottom: "0.5rem",
          padding: "0.5rem 0", borderBottom: "1px solid #e5e7eb" }}>
          Asked by {viewingHistoryEntry.asked_by} on{" "}
          {new Date(viewingHistoryEntry.asked_at).toLocaleString()} · {viewingHistoryEntry.model}
        </div>
      )}

      {/* Answer display */}
      {response && <AnswerDisplay response={response} />}

      {/* Follow Up button */}
      {canFollowUp && (
        <button onClick={handleFollowUp} style={{
          marginTop: "0.75rem", padding: "0.5rem 1rem", backgroundColor: "transparent",
          border: "1px solid #2563eb", color: "#2563eb", borderRadius: "4px",
          cursor: "pointer", fontSize: "0.85rem",
        }}>
          {pageText.ask.followUpButton}
        </button>
      )}
      </>)}

      {/* History tab */}
      {activeTab === "history" && (
        <div>
          {historyLoading && (
            <div style={{ textAlign: "center", color: "#6b7280", padding: "2rem 0" }}>
              Loading…
            </div>
          )}
          {!historyLoading && history.length === 0 && (
            <div style={{ textAlign: "center", color: "#6b7280", padding: "2rem 0" }}>
              No questions yet.
            </div>
          )}
          {!historyLoading && history.map((entry) => (
            <HistoryCard
              key={entry.id}
              entry={entry}
              onClick={(id) => handleHistoryClick(id)}
              onRate={(rating) => rateQAEntry(entry.id, rating)}
              onDelete={handleDelete}
            />
          ))}
        </div>
      )}
    </div>
  );
};

export default AskPage;
