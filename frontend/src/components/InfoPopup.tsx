import React, { useState, useRef, useEffect } from "react";

type InfoPopupProps = {
  children: React.ReactNode;
};

/** A small ⓘ icon that toggles a popup on click. Click outside to dismiss. */
const InfoPopup: React.FC<InfoPopupProps> = ({ children }) => {
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    const handler = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) setOpen(false);
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, [open]);

  return (
    <span ref={ref} style={{ position: "relative", display: "inline-block", marginLeft: "0.5rem" }}>
      <button
        onClick={() => setOpen((v) => !v)}
        style={{
          background: "none", border: "1px solid #d1d5db", borderRadius: "50%",
          width: "1.4rem", height: "1.4rem", cursor: "pointer", fontSize: "0.8rem",
          color: "#6b7280", lineHeight: 1, display: "inline-flex", alignItems: "center",
          justifyContent: "center", verticalAlign: "middle",
        }}
        title="How evidence strength is calculated"
      >
        i
      </button>
      {open && (
        <div style={{
          position: "absolute", top: "2rem", left: 0, zIndex: 100,
          width: "360px", padding: "1rem 1.25rem", backgroundColor: "#ffffff",
          border: "1px solid #e5e7eb", borderRadius: "8px",
          boxShadow: "0 4px 12px rgba(0,0,0,0.12)", fontSize: "0.85rem",
          color: "#374151", lineHeight: 1.6,
        }}>
          {children}
        </div>
      )}
    </span>
  );
};

export default InfoPopup;
