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
    disparagement: { bg: "#fee2e2", text: "#991b1b" },
    selective_enforcement: { bg: "#ffedd5", text: "#9a3412" },
    financial_misconduct: { bg: "#fef3c7", text: "#92400e" },
    secrecy: { bg: "#e0e7ff", text: "#3730a3" },
    coordination: { bg: "#dbeafe", text: "#1e40af" },
    lies_under_oath: { bg: "#fce7f3", text: "#9d174d" },
    conflict_of_interest: { bg: "#f3e8ff", text: "#6b21a8" },
    evasive_responses: { bg: "#dcfce7", text: "#166534" },
    judicial_bias: { bg: "#ede9fe", text: "#5b21b6" },
    due_process_violation: { bg: "#fef9c3", text: "#854d0e" },
};

const FALLBACK_TAG_PILL = { bg: "#f1f5f9", text: "#334155" };

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
    backgroundColor: "#ffffff",
    border: "1px solid #e2e8f0",
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
    color: "#1d4ed8",
};

const titleStyle: React.CSSProperties = {
    fontSize: "0.95rem",
    fontWeight: 700,
    color: "#0f172a",
    marginBottom: "0.4rem",
};

const quoteWrapBase: React.CSSProperties = {
    fontFamily: "Georgia, serif",
    fontStyle: "italic",
    fontSize: "0.88rem",
    color: "#334155",
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
    color: "#1d4ed8",
    cursor: "pointer",
    fontWeight: 500,
};

const footerRow: React.CSSProperties = {
    display: "flex",
    flexWrap: "wrap",
    alignItems: "center",
    gap: "0.45rem",
    fontSize: "0.78rem",
    color: "#64748b",
    marginTop: "0.4rem",
};

const docTitleStyle: React.CSSProperties = {
    fontWeight: 600,
    color: "#334155",
};

const pdfBtnStyle: React.CSSProperties = {
    padding: "0.2rem 0.55rem",
    fontSize: "0.74rem",
    fontWeight: 500,
    border: "1px solid #cbd5e1",
    borderRadius: "5px",
    backgroundColor: "#f8fafc",
    color: "#1d4ed8",
    textDecoration: "none",
    fontFamily: "inherit",
};

const aboutLineStyle: React.CSSProperties = {
    fontSize: "0.78rem",
    color: "#475569",
    marginTop: "0.4rem",
};

// ─── Component ──────────────────────────────────────────────────────────────

interface Props {
    instance: BiasInstance;
}

const EvidenceCard: React.FC<Props> = ({ instance }) => {
    const [expanded, setExpanded] = useState(false);

    const speakerName = instance.stated_by?.name;
    const aboutNames = instance.about.map((s) => s.name).filter((n) => n && n.length > 0);

    // Build the View-in-PDF target. We pass `tab=document` so the
    // DocumentWorkspace lands on the PDF viewer when arriving from here.
    // The DocumentWorkspace today reads `?page=` from the URL; if it does
    // not yet honor `tab=`, the link still works (the tab parameter is
    // ignored harmlessly).
    let pdfHref: string | null = null;
    if (instance.document) {
        const params = new URLSearchParams();
        if (instance.page_number != null) {
            params.set("page", String(instance.page_number));
        }
        params.set("tab", "document");
        const qs = params.toString();
        pdfHref = `/documents/${instance.document.id}${qs ? "?" + qs : ""}`;
    }

    return (
        <div style={cardStyle}>
            {/* Header — speaker + pattern chips */}
            <div style={headerRow}>
                {speakerName && <span style={speakerStyle}>{speakerName}</span>}
                {instance.pattern_tags.map((t) => (
                    <span key={t} style={tagPillStyle(t)} title={t}>
                        {formatTagLabel(t)}
                    </span>
                ))}
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

            {/* Footer — document + page + PDF link */}
            {instance.document && (
                <div style={footerRow}>
                    <span style={docTitleStyle}>{instance.document.title}</span>
                    {instance.page_number != null && <span>p.{instance.page_number}</span>}
                    {pdfHref && (
                        <Link to={pdfHref} style={pdfBtnStyle}>
                            View in PDF
                        </Link>
                    )}
                </div>
            )}
        </div>
    );
};

export default EvidenceCard;

export { formatTagLabel };
