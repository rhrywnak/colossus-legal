/**
 * PdfViewer — Renders a PDF document with page navigation.
 *
 * Uses react-pdf (wrapper around Mozilla's PDF.js) to render PDF pages
 * inline in the browser. The component is controlled: the parent specifies
 * which page to show via the `page` prop, and receives page change events
 * via `onPageChange`.
 *
 * REACT LEARNING — Controlled vs Uncontrolled Components:
 * This component follows the "controlled component" pattern. The parent
 * owns the page state and passes it down as a prop. When the user clicks
 * next/prev, we don't update internal state — we call onPageChange() and
 * let the parent decide the new page. This enables the Document Workspace
 * to sync the PDF page with the selected evidence card.
 */
import React, { useCallback, useEffect, useRef, useState } from "react";
import { Document, Page, pdfjs } from "react-pdf";
import "react-pdf/dist/Page/AnnotationLayer.css";
import "react-pdf/dist/Page/TextLayer.css";

// Configure PDF.js worker — MUST be in the same file as Document/Page usage
pdfjs.GlobalWorkerOptions.workerSrc = new URL(
  "pdfjs-dist/build/pdf.worker.min.mjs",
  import.meta.url,
).toString();

interface PdfViewerProps {
  /** URL to the PDF file (relative or absolute) */
  src: string;
  /** Current page number (1-indexed). Default: 1 */
  page?: number;
  /** Callback when page changes (user navigation) */
  onPageChange?: (page: number) => void;
  /** Optional CSS class for outer container */
  className?: string;
}

const toolbarStyle: React.CSSProperties = {
  display: "flex", alignItems: "center", justifyContent: "center",
  gap: "0.5rem", padding: "0.5rem 0.75rem",
  backgroundColor: "#f8fafc", borderBottom: "1px solid #e2e8f0",
  fontSize: "0.82rem", color: "#475569",
};

const navBtnStyle: React.CSSProperties = {
  padding: "0.25rem 0.6rem", fontSize: "0.8rem", fontWeight: 500,
  border: "1px solid #e2e8f0", borderRadius: "4px",
  backgroundColor: "#fff", color: "#334155", cursor: "pointer",
  fontFamily: "inherit",
};

const pageInputStyle: React.CSSProperties = {
  width: "3rem", textAlign: "center", padding: "0.2rem 0.3rem",
  border: "1px solid #e2e8f0", borderRadius: "4px",
  fontSize: "0.82rem", fontFamily: "inherit",
};

const PdfViewer: React.FC<PdfViewerProps> = ({
  src, page = 1, onPageChange, className,
}) => {
  const [numPages, setNumPages] = useState<number | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [pageInput, setPageInput] = useState(String(page));
  const [containerWidth, setContainerWidth] = useState<number>(600);
  const containerRef = useRef<HTMLDivElement>(null);

  // Sync pageInput when page prop changes
  useEffect(() => { setPageInput(String(page)); }, [page]);

  // Track container width with ResizeObserver for fit-to-width rendering
  useEffect(() => {
    const el = containerRef.current;
    if (!el) return;
    const observer = new ResizeObserver((entries) => {
      for (const entry of entries) {
        setContainerWidth(entry.contentRect.width);
      }
    });
    observer.observe(el);
    setContainerWidth(el.clientWidth);
    return () => observer.disconnect();
  }, []);

  const onLoadSuccess = useCallback(({ numPages: n }: { numPages: number }) => {
    setNumPages(n);
    setLoading(false);
    setError(null);
  }, []);

  const onLoadError = useCallback((err: Error) => {
    setError(`Failed to load PDF: ${err.message}`);
    setLoading(false);
  }, []);

  const goToPage = (p: number) => {
    if (p >= 1 && p <= (numPages ?? 1) && onPageChange) onPageChange(p);
  };

  const handlePageInputBlur = () => {
    const n = parseInt(pageInput, 10);
    if (!isNaN(n) && n >= 1 && n <= (numPages ?? 1)) {
      goToPage(n);
    } else {
      setPageInput(String(page));
    }
  };

  const handlePageInputKey = (e: React.KeyboardEvent) => {
    if (e.key === "Enter") handlePageInputBlur();
  };

  return (
    <div
      ref={containerRef}
      className={className}
      style={{ border: "1px solid #e2e8f0", borderRadius: "8px", overflow: "hidden", backgroundColor: "#fff" }}
    >
      {/* Toolbar */}
      <div style={toolbarStyle}>
        <button style={navBtnStyle} disabled={page <= 1} onClick={() => goToPage(page - 1)}>
          Prev
        </button>
        <span>Page</span>
        <input
          style={pageInputStyle}
          value={pageInput}
          onChange={(e) => setPageInput(e.target.value)}
          onBlur={handlePageInputBlur}
          onKeyDown={handlePageInputKey}
        />
        <span>of {numPages ?? "..."}</span>
        <button style={navBtnStyle} disabled={page >= (numPages ?? 1)} onClick={() => goToPage(page + 1)}>
          Next
        </button>
      </div>

      {/* PDF render area */}
      <div style={{ minHeight: "400px", display: "flex", justifyContent: "center" }}>
        {error ? (
          <div style={{ padding: "2rem", color: "#dc2626", fontSize: "0.84rem", textAlign: "center" }}>
            {error}<br />
            <span style={{ color: "#64748b", fontSize: "0.76rem" }}>URL: {src}</span>
          </div>
        ) : (
          <Document
            file={src}
            onLoadSuccess={onLoadSuccess}
            onLoadError={onLoadError}
            loading={
              <div style={{ padding: "3rem", color: "#64748b", fontSize: "0.84rem" }}>
                Loading PDF...
              </div>
            }
          >
            {!loading && (
              <Page
                pageNumber={page}
                width={containerWidth > 0 ? containerWidth : undefined}
                loading={
                  <div style={{ padding: "2rem", color: "#64748b", fontSize: "0.82rem" }}>
                    Rendering page {page}...
                  </div>
                }
              />
            )}
          </Document>
        )}
      </div>
    </div>
  );
};

export default PdfViewer;
