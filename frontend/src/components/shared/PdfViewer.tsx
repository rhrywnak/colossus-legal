/**
 * PdfViewer — Domain-agnostic PDF viewer with continuous scroll, zoom, and text highlighting.
 *
 * Reusable across projects: depends only on react-pdf and the generic highlight utilities.
 * Uses IntersectionObserver to track visible page and isScrollingToPage ref to prevent
 * feedback loops between scroll-to-page and page-change reporting.
 */
import React, { useCallback, useEffect, useRef, useState } from "react";
import { Document, Page, pdfjs } from "react-pdf";
import "react-pdf/dist/Page/AnnotationLayer.css";
import "react-pdf/dist/Page/TextLayer.css";
import { HIGHLIGHT_DEFAULTS } from "../../utils/highlightConstants";
import { clearHighlights, highlightTextOnPage } from "../../utils/pdfHighlight";

pdfjs.GlobalWorkerOptions.workerSrc = new URL(
  "pdfjs-dist/build/pdf.worker.min.mjs",
  import.meta.url,
).toString();

interface PdfViewerProps {
  src: string;
  page?: number;
  onPageChange?: (page: number) => void;
  className?: string;
  highlightText?: string | null;
  highlightColor?: string;
  highlightPage?: number | null;
}

const toolbarStyle: React.CSSProperties = {
  display: "flex", alignItems: "center", justifyContent: "center",
  gap: "0.5rem", padding: "0.5rem 0.75rem",
  backgroundColor: "#f8fafc", borderBottom: "1px solid #e2e8f0",
  fontSize: "0.82rem", color: "#475569",
  position: "sticky", top: 0, zIndex: 10,
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

const ZOOM_STEPS = [0.5, 0.75, 1.0, 1.25, 1.5, 2.0];

const zoomBtnStyle: React.CSSProperties = {
  ...navBtnStyle,
  padding: "0.25rem 0.45rem",
  fontSize: "0.75rem",
  minWidth: "1.6rem",
};

const PdfViewer: React.FC<PdfViewerProps> = ({
  src, page = 1, onPageChange, className,
  highlightText, highlightColor = HIGHLIGHT_DEFAULTS.color, highlightPage,
}) => {
  const [numPages, setNumPages] = useState<number | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [pageInput, setPageInput] = useState(String(page));
  const [containerWidth, setContainerWidth] = useState<number>(600);
  const [zoomLevel, setZoomLevel] = useState(1.0);
  const containerRef = useRef<HTMLDivElement>(null);
  const scrollRef = useRef<HTMLDivElement>(null);
  const isScrollingToPage = useRef(false);
  const scrollTimeout = useRef<ReturnType<typeof setTimeout>>();

  useEffect(() => { setPageInput(String(page)); }, [page]);

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

  useEffect(() => {
    const scrollEl = scrollRef.current;
    if (!scrollEl || !numPages || loading) return;

    const observer = new IntersectionObserver(
      (entries) => {
        if (isScrollingToPage.current) return;
        let maxRatio = 0;
        let visiblePage = page;
        entries.forEach((entry) => {
          if (entry.intersectionRatio > maxRatio) {
            maxRatio = entry.intersectionRatio;
            const pn = Number(entry.target.getAttribute("data-page-number"));
            if (pn) visiblePage = pn;
          }
        });
        if (visiblePage !== page && onPageChange) {
          onPageChange(visiblePage);
        }
      },
      { root: scrollEl, threshold: [0, 0.25, 0.5, 0.75, 1.0] },
    );

    const pages = scrollEl.querySelectorAll("[data-page-number]");
    pages.forEach((el) => observer.observe(el));
    return () => observer.disconnect();
  }, [numPages, loading, page, onPageChange]);

  useEffect(() => {
    const scrollEl = scrollRef.current;
    if (!scrollEl || !numPages || loading) return;
    const pageEl = scrollEl.querySelector(`[data-page-number="${page}"]`);
    if (!pageEl) return;

    isScrollingToPage.current = true;
    pageEl.scrollIntoView({ behavior: "smooth", block: "start" });

    clearTimeout(scrollTimeout.current);
    scrollTimeout.current = setTimeout(() => {
      isScrollingToPage.current = false;
    }, 500);
  }, [page, numPages, loading]);

  useEffect(() => {
    return () => clearTimeout(scrollTimeout.current);
  }, []);

  const handleTextLayerReady = useCallback((pageNum: number) => {
    const scrollEl = scrollRef.current;
    if (!scrollEl || !highlightText || highlightPage !== pageNum) return;
    clearHighlights(scrollEl);
    highlightTextOnPage(scrollEl, pageNum, highlightText, highlightColor);
  }, [highlightText, highlightPage, highlightColor]);

  useEffect(() => {
    const scrollEl = scrollRef.current;
    if (!scrollEl) return;
    clearHighlights(scrollEl);
    if (highlightText && highlightPage) {
      highlightTextOnPage(scrollEl, highlightPage, highlightText, highlightColor);
    }
  }, [highlightText, highlightPage, highlightColor]);

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

  const zoomIn = () => {
    const next = ZOOM_STEPS.find((s) => s > zoomLevel);
    if (next) setZoomLevel(next);
  };

  const zoomOut = () => {
    const prev = [...ZOOM_STEPS].reverse().find((s) => s < zoomLevel);
    if (prev) setZoomLevel(prev);
  };

  const zoomFit = () => setZoomLevel(1.0);

  const effectiveWidth = containerWidth > 0 ? containerWidth * zoomLevel : undefined;

  return (
    <div
      ref={containerRef}
      className={className}
      style={{ display: "flex", flexDirection: "column", height: "100%", backgroundColor: "#fff" }}
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

        <span style={{ borderLeft: "1px solid #e2e8f0", height: "1.2rem", margin: "0 0.25rem" }} />

        <button style={zoomBtnStyle} disabled={zoomLevel <= ZOOM_STEPS[0]} onClick={zoomOut}>
          −
        </button>
        <span style={{ fontSize: "0.78rem", minWidth: "2.8rem", textAlign: "center" }}>
          {Math.round(zoomLevel * 100)}%
        </span>
        <button style={zoomBtnStyle} disabled={zoomLevel >= ZOOM_STEPS[ZOOM_STEPS.length - 1]} onClick={zoomIn}>
          +
        </button>
        <button style={{ ...zoomBtnStyle, fontSize: "0.72rem" }} disabled={zoomLevel === 1.0} onClick={zoomFit}>
          Fit
        </button>
      </div>

      <div
        ref={scrollRef}
        style={{
          flex: 1,
          overflowY: "auto",
          overflowX: zoomLevel > 1.0 ? "auto" : "hidden",
          backgroundColor: "#f1f5f9",
        }}
      >
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
            {!loading && numPages && Array.from({ length: numPages }, (_, i) => i + 1).map((pn) => (
              <div
                key={pn}
                data-page-number={pn}
                style={{
                  marginBottom: "8px",
                  display: "flex",
                  justifyContent: zoomLevel > 1.0 ? "flex-start" : "center",
                }}
              >
                <Page
                  pageNumber={pn}
                  width={effectiveWidth}
                  renderTextLayer={true}
                  onRenderTextLayerSuccess={() => handleTextLayerReady(pn)}
                  loading={
                    <div style={{ padding: "2rem", color: "#64748b", fontSize: "0.82rem" }}>
                      Rendering page {pn}...
                    </div>
                  }
                />
              </div>
            ))}
          </Document>
        )}
      </div>
    </div>
  );
};

export default PdfViewer;
