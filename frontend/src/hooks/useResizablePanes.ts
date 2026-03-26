/**
 * useResizablePanes — Hook for draggable split-pane divider.
 *
 * Returns the current split percentage and props for the divider element.
 * Drag is handled via document-level pointer events so the cursor doesn't
 * lose the divider during fast mouse movement.
 */
import { useCallback, useEffect, useRef, useState } from "react";

interface UseResizablePanesOptions {
  defaultPercent?: number;
  minPercent?: number;
  maxPercent?: number;
}

interface DividerProps {
  onMouseDown: () => void;
  style: React.CSSProperties;
}

interface UseResizablePanesResult {
  splitPercent: number;
  containerRef: React.RefObject<HTMLDivElement>;
  dividerProps: DividerProps;
  isDragging: boolean;
}

const dividerBaseStyle: React.CSSProperties = {
  width: "6px",
  cursor: "col-resize",
  backgroundColor: "#e2e8f0",
  display: "flex",
  alignItems: "center",
  justifyContent: "center",
  flexShrink: 0,
  transition: "background-color 0.15s",
};

export function useResizablePanes({
  defaultPercent = 55,
  minPercent = 30,
  maxPercent = 70,
}: UseResizablePanesOptions = {}): UseResizablePanesResult {
  const [splitPercent, setSplitPercent] = useState(defaultPercent);
  const dragging = useRef(false);
  const [isDragging, setIsDragging] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null!);  // non-null assert for legacy ref compat

  const handleMouseDown = useCallback(() => {
    dragging.current = true;
    setIsDragging(true);
  }, []);

  useEffect(() => {
    const handleMouseMove = (e: MouseEvent) => {
      if (!dragging.current || !containerRef.current) return;
      const rect = containerRef.current.getBoundingClientRect();
      const percent = ((e.clientX - rect.left) / rect.width) * 100;
      setSplitPercent(Math.min(maxPercent, Math.max(minPercent, percent)));
    };

    const handleMouseUp = () => {
      if (dragging.current) {
        dragging.current = false;
        setIsDragging(false);
      }
    };

    document.addEventListener("mousemove", handleMouseMove);
    document.addEventListener("mouseup", handleMouseUp);
    return () => {
      document.removeEventListener("mousemove", handleMouseMove);
      document.removeEventListener("mouseup", handleMouseUp);
    };
  }, [minPercent, maxPercent]);

  return {
    splitPercent,
    containerRef,
    dividerProps: {
      onMouseDown: handleMouseDown,
      style: {
        ...dividerBaseStyle,
        backgroundColor: isDragging ? "#94a3b8" : "#e2e8f0",
      },
    },
    isDragging,
  };
}
