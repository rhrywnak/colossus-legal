// Bias Explorer — Evidence card.
//
// Bias-tailored card for a single Evidence node. Distinct from the v1
// admin EvidenceCard (which renders DocumentEvidence rows for a deprecated
// import workflow) and from the pipeline ContentPanel (which renders
// extraction-staging items, not graph nodes). We deliberately keep this
// presentation-only and local to the BiasExplorer module so the v1 admin
// card can be removed without touching this code, and vice versa.
//
// Layout matches the instruction §5.4:
//   - Header   : speaker name + pattern-tag chips
//   - Title    : evidence title (bold)
//   - Quote    : verbatim quote, italic, ~3-line clamp with Show more
//   - Footer   : "Document title — p.N — [View in PDF]"
//   - Subjects : "About: ..." line if non-empty

import React, { useState } from "react";
import { Link } from "react-router-dom";

import type { BiasInstance } from "./types";

// ─── Pattern-tag pill colors ────────────────────────────────────────────────
//
// Per Standing Rule 2, pattern tag *values* are never hardcoded into business
// logic. Choosing a pill color from the tag is presentation-only — if a tag
// is unknown to the palette, we fall back to a neutral slate. Adding a new
// tag does not require touching this file (the fallback handles it).

const TAG_PALETTE: Record<string, { bg: string; text: string }> = {
    disparagement: { bg: "var(--state-danger-bg-soft)", text: "var(--status-dropped-text)" },
    selective_enforcement: { bg: "var(--burden-warning-bg)", text: "var(--burden-warning-text)" },
    financial_misconduct: { bg: "var(--burden-warning-bg)", text: "var(--burden-warning-text)" },
    secrecy: { bg: "var(--state-info-bg-soft)", text: "var(--bias-indigo-text)" },
    coordination: { bg: "var(--accent-bg-soft)", text: "var(--accent-primary-hover)" },
    lies_under_oath: { bg: "var(--bias-pink-bg-soft)", text: "var(--bias-pink-text)" },
    conflict_of_interest: { bg: "var(--bias-purple-bg-soft)", text: "var(--bias-purple-text)" },
    evasive_responses: { bg: "var(--state-success-bg-soft)", text: "var(--status-active-text)" },
    judicial_bias: { bg: "var(--bias-purple-bg-soft)", text: "var(--bias-purple-text)" },
    due_process_violation: { bg: "var(--burden-warning-bg)", text: "var(--burden-warning-text)" },
};

const FALLBACK_TAG_PILL = { bg: "var(--bg-page)", text: "var(--text-secondary)" };

function tagPillStyle(tag: string): React.CSSProperties {
    const palette = TAG_PALETTE[tag] ?? FALLBACK_TAG_PILL;
    return {
        padding: "0.15rem 0.55rem",
        borderRadius: "9999px",
        fontSize: "0.7rem",
        fontWeight: 600,
        backgroundColor: palette.bg,
        color: palette.text,
        whiteSpace: "nowrap",
    };
}

function formatTagLabel(raw: string): string {
    return raw
        .split("_")
        .map((p) => (p.length === 0 ? p : p[0].toUpperCase() + p.slice(1)))
        .join(" ");
}

// ─── Styles ─────────────────────────────────────────────────────────────────

const cardStyle: React.CSSProperties = {
    backgroundColor: "var(--bg-surface)",
    border: "1px solid var(--border-default)",
    borderRadius: "8px",
    padding: "0.85rem 1rem",
    boxShadow: "0 1px 2px rgba(0,0,0,0.04)",
};

const headerRow: React.CSSProperties = {
    display: "flex",
    flexWrap: "wrap",
    alignItems: "center",
    gap: "0.4rem",
    marginBottom: "0.4rem",
};

const speakerStyle: React.CSSProperties = {
    fontSize: "0.82rem",
    fontWeight: 600,
    color: "var(--accent-primary)",
};

const titleStyle: React.CSSProperties = {
    fontSize: "0.95rem",
    fontWeight: 700,
    color: "var(--text-primary)",
    marginBottom: "0.4rem",
};

const quoteWrapBase: React.CSSProperties = {
    fontFamily: "Georgia, serif",
    fontStyle: "italic",
    fontSize: "0.88rem",
    color: "var(--text-secondary)",
    lineHeight: 1.55,
    marginBottom: "0.45rem",
    whiteSpace: "pre-wrap",
};

const clampStyle: React.CSSProperties = {
    ...quoteWrapBase,
    display: "-webkit-box",
    WebkitLineClamp: 3,
    WebkitBoxOrient: "vertical",
    overflow: "hidden",
};

const showMoreLink: React.CSSProperties = {
    background: "none",
    border: "none",
    padding: 0,
    fontFamily: "inherit",
    fontSize: "0.8rem",
    color: "var(--accent-primary)",
    cursor: "pointer",
    fontWeight: 500,
};

const footerRow: React.CSSProperties = {
    display: "flex",
    flexWrap: "wrap",
    alignItems: "center",
    gap: "0.45rem",
    fontSize: "0.78rem",
    color: "var(--text-muted)",
    marginTop: "0.4rem",
};

const docTitleStyle: React.CSSProperties = {
    fontWeight: 600,
    color: "var(--text-secondary)",
};

const pdfBtnStyle: React.CSSProperties = {
    padding: "0.2rem 0.55rem",
    fontSize: "0.74rem",
    fontWeight: 500,
    border: "1px solid var(--border-default)",
    borderRadius: "5px",
    backgroundColor: "var(--bg-page)",
    color: "var(--accent-primary)",
    textDecoration: "none",
    fontFamily: "inherit",
};

// The SAME chrome as `pdfBtnStyle`, but for the `<button>` variant used when a
// host supplies `onViewPdf` (open-in-place instead of navigate-away). A native
// button needs `cursor: pointer` and a `0` line-height reset the anchor got for
// free; everything else is shared so the two variants are visually identical.
const pdfBtnAsButtonStyle: React.CSSProperties = {
    ...pdfBtnStyle,
    cursor: "pointer",
    lineHeight: "normal",
};

const aboutLineStyle: React.CSSProperties = {
    fontSize: "0.78rem",
    color: "var(--text-secondary)",
    marginTop: "0.4rem",
};

// ─── Component ──────────────────────────────────────────────────────────────

/**
 * The minimum a host needs to open a candidate's source PDF in its own viewer.
 * Emitted by the "View in PDF" control when a host supplies [`Props.onViewPdf`].
 * Deliberately NOT a URL: the card stays agnostic about WHERE the file is served
 * (which endpoint, which base URL) — the host builds that from the id.
 */
export interface ViewPdfTarget {
    documentId: string;
    documentTitle: string;
    /** 1-based source page, or `null` when the evidence carries no page. */
    page: number | null;
    /** The verbatim quote, so the host's viewer can highlight it in place; `null`
     *  when the evidence has no quote to anchor on. */
    highlightText: string | null;
}

interface Props {
    instance: BiasInstance;
    /**
     * Optional action rendered at the right of the header row — e.g. an
     * "Add to scenario" button on a bias candidate, or a "Remove" button on a
     * saved scenario fact. Kept as an opaque slot so this card stays
     * presentation-only: it knows nothing about scenarios or curation, and the
     * same card renders both a candidate and a saved fact (one card, two uses).
     */
    action?: React.ReactNode;
    /**
     * Optional "view the source in place" handler. When PROVIDED, the footer's
     * "View in PDF" control becomes a `<button>` that calls this with a
     * [`ViewPdfTarget`] — so a host (the scenario workbench) can open the PDF in
     * its own side-panel viewer WITHOUT a route change. When OMITTED, the control
     * stays the default `<Link>` that navigates to the document workspace, so the
     * Bias Explorer and Theme Scan consumers are entirely unchanged.
     *
     * ## React Learning: additive optional prop = zero-touch extension
     * Making this optional and branching on its presence means the two existing
     * callers need no edit and keep their exact behavior; only the workbench,
     * which passes it, opts into the new path. This is the same "the host decides
     * its own presentation, the shared card stays agnostic" pattern the workbench
     * already uses for tag demotion — extend by prop, never fork the component.
     */
    onViewPdf?: (target: ViewPdfTarget) => void;
}

const EvidenceCard: React.FC<Props> = ({ instance, action, onViewPdf }) => {
    const [expanded, setExpanded] = useState(false);

    const speakerName = instance.stated_by?.name;
    const aboutNames = instance.about.map((s) => s.name).filter((n) => n && n.length > 0);

    // Capture the document once as a `const` so its non-null narrowing flows into
    // the footer's button `onClick` closure below (TS narrows a `const` across
    // closures, which it cannot do for `instance.document` re-reads).
    const doc = instance.document;

    // Build the default navigate-away target (used only when `onViewPdf` is NOT
    // supplied). We pass `tab=document` so the DocumentWorkspace lands on the PDF
    // viewer when arriving from here. The DocumentWorkspace today reads `?page=`
    // from the URL; if it does not yet honor `tab=`, the link still works (the tab
    // parameter is ignored harmlessly).
    let pdfHref: string | null = null;
    if (doc) {
        const params = new URLSearchParams();
        if (instance.page_number != null) {
            params.set("page", String(instance.page_number));
        }
        params.set("tab", "document");
        const qs = params.toString();
        pdfHref = `/documents/${doc.id}${qs ? "?" + qs : ""}`;
    }

    return (
        <div style={cardStyle}>
            {/* Header — speaker + pattern chips, plus an optional action at the
                far right (pushed there by margin-left:auto on its wrapper). */}
            <div style={headerRow}>
                {speakerName && <span style={speakerStyle}>{speakerName}</span>}
                {instance.pattern_tags.map((t) => (
                    <span key={t} style={tagPillStyle(t)} title={t}>
                        {formatTagLabel(t)}
                    </span>
                ))}
                {action && <span style={{ marginLeft: "auto" }}>{action}</span>}
            </div>

            {/* Title */}
            {instance.title && <div style={titleStyle}>{instance.title}</div>}

            {/* Quote */}
            {instance.verbatim_quote && (
                <>
                    <div style={expanded ? quoteWrapBase : clampStyle}>
                        &ldquo;{instance.verbatim_quote}&rdquo;
                    </div>
                    <button
                        type="button"
                        style={showMoreLink}
                        onClick={() => setExpanded((v) => !v)}
                    >
                        {expanded ? "Show less" : "Show more"}
                    </button>
                </>
            )}

            {/* About line */}
            {aboutNames.length > 0 && (
                <div style={aboutLineStyle}>
                    <strong>About:</strong> {aboutNames.join(", ")}
                </div>
            )}

            {/* Footer — document + page + PDF control. When a host supplies
                `onViewPdf`, the control opens the source in place (a button that
                calls back); otherwise it stays the default navigate-away `<Link>`
                for the Bias Explorer / Theme Scan consumers (unchanged). */}
            {doc && (
                <div style={footerRow}>
                    <span style={docTitleStyle}>{doc.title}</span>
                    {instance.page_number != null && <span>p.{instance.page_number}</span>}
                    {onViewPdf ? (
                        <button
                            type="button"
                            style={pdfBtnAsButtonStyle}
                            onClick={() =>
                                onViewPdf({
                                    documentId: doc.id,
                                    documentTitle: doc.title,
                                    page: instance.page_number ?? null,
                                    highlightText: instance.verbatim_quote ?? null,
                                })
                            }
                        >
                            View in PDF
                        </button>
                    ) : (
                        pdfHref && (
                            <Link to={pdfHref} style={pdfBtnStyle}>
                                View in PDF
                            </Link>
                        )
                    )}
                </div>
            )}
        </div>
    );
};

export default EvidenceCard;

export { formatTagLabel };
