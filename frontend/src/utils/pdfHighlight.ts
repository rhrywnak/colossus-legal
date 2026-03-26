/**
 * PDF text-layer search and highlight utility.
 *
 * Domain-agnostic: works with any react-pdf text layer.
 * Finds text across span boundaries and applies highlight styling.
 */
import { HIGHLIGHT_ATTRS, HIGHLIGHT_DEFAULTS } from "./highlightConstants";

export interface HighlightResult {
  found: boolean;
  pageNumber: number | null;
}

interface SpanRange {
  span: HTMLElement;
  startIdx: number;
  endIdx: number;
}

function normalizeText(text: string): string {
  return text.replace(/\s+/g, " ").trim().toLowerCase();
}

/**
 * Apply highlight styling to a span (full or partial).
 *
 * For full-span matches, sets background-color directly.
 * For partial matches, wraps the matching portion in a <mark> element.
 * Opacity is applied from HIGHLIGHT_DEFAULTS so it's configurable in one place.
 */
function applyHighlightToSpan(
  span: HTMLElement,
  startInSpan: number,
  endInSpan: number,
  color: string,
): void {
  const text = span.textContent || "";
  const opacity = String(HIGHLIGHT_DEFAULTS.opacity);

  if (startInSpan === 0 && endInSpan === text.length) {
    span.style.backgroundColor = color;
    span.style.opacity = opacity;
    span.setAttribute(HIGHLIGHT_ATTRS.dataAttribute, HIGHLIGHT_ATTRS.markedValue);
  } else {
    const before = text.substring(0, startInSpan);
    const match = text.substring(startInSpan, endInSpan);
    const after = text.substring(endInSpan);

    span.textContent = "";
    if (before) span.appendChild(document.createTextNode(before));

    const mark = document.createElement("mark");
    mark.style.backgroundColor = color;
    mark.style.color = "transparent";
    mark.style.opacity = opacity;
    mark.setAttribute(HIGHLIGHT_ATTRS.dataAttribute, HIGHLIGHT_ATTRS.markedValue);
    mark.textContent = match;
    span.appendChild(mark);

    if (after) span.appendChild(document.createTextNode(after));
  }
}

/**
 * Search the text layer of a specific page for the given text and apply
 * a highlight background to the matching spans.
 *
 * Works across span boundaries — a quote that spans two lines in the PDF
 * will highlight correctly across both spans.
 */
export function highlightTextOnPage(
  scrollContainer: HTMLElement,
  pageNumber: number,
  searchText: string,
  color: string,
): HighlightResult {
  const pageEl = scrollContainer.querySelector(`[data-page-number="${pageNumber}"]`);
  if (!pageEl) return { found: false, pageNumber };

  const textLayer = pageEl.querySelector(".react-pdf__Page__textContent");
  if (!textLayer) return { found: false, pageNumber };

  const spans = Array.from(textLayer.querySelectorAll("span")) as HTMLElement[];
  if (spans.length === 0) return { found: false, pageNumber };

  // Build concatenated text with span position tracking
  const spanRanges: SpanRange[] = [];
  let fullText = "";

  for (const span of spans) {
    const text = span.textContent || "";
    const startIdx = fullText.length;
    fullText += text + " ";
    spanRanges.push({ span, startIdx, endIdx: startIdx + text.length });
  }

  const normalizedFull = normalizeText(fullText);
  const normalizedSearch = normalizeText(searchText);

  if (!normalizedSearch) return { found: false, pageNumber };

  const matchIdx = normalizedFull.indexOf(normalizedSearch);
  if (matchIdx === -1) return { found: false, pageNumber };

  const matchEnd = matchIdx + normalizedSearch.length;

  // Highlight spans that overlap the match range
  for (const range of spanRanges) {
    const overlapStart = Math.max(matchIdx, range.startIdx);
    const overlapEnd = Math.min(matchEnd, range.endIdx);

    if (overlapStart < overlapEnd) {
      applyHighlightToSpan(
        range.span,
        overlapStart - range.startIdx,
        overlapEnd - range.startIdx,
        color,
      );
    }
  }

  return { found: true, pageNumber };
}

/**
 * Clear all existing highlights in the scroll container.
 *
 * Removes background-color from highlighted spans and replaces
 * <mark> elements with their text content.
 */
export function clearHighlights(container: HTMLElement): void {
  const selector = `[${HIGHLIGHT_ATTRS.dataAttribute}="${HIGHLIGHT_ATTRS.markedValue}"]`;
  container.querySelectorAll(selector).forEach((el) => {
    if (el.tagName === "MARK") {
      const parent = el.parentNode;
      if (parent) {
        parent.replaceChild(document.createTextNode(el.textContent || ""), el);
        parent.normalize();
      }
    } else {
      (el as HTMLElement).style.backgroundColor = "";
      (el as HTMLElement).style.opacity = "";
      el.removeAttribute(HIGHLIGHT_ATTRS.dataAttribute);
    }
  });
}
