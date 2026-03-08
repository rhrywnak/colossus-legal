import { useState } from "react";

interface Props {
  value: number | null;
  onChange: (rating: number) => void;
}

export function StarRating({ value, onChange }: Props) {
  const [hovered, setHovered] = useState<number | null>(null);
  const display = hovered ?? value ?? 0;

  return (
    <div
      style={{ display: "inline-flex", gap: "2px" }}
      onClick={(e) => e.stopPropagation()}
    >
      {[1, 2, 3, 4, 5].map((star) => (
        <span
          key={star}
          onMouseEnter={() => setHovered(star)}
          onMouseLeave={() => setHovered(null)}
          onClick={() => onChange(star)}
          role="button"
          aria-label={`Rate ${star} star${star > 1 ? "s" : ""}`}
          style={{
            cursor: "pointer",
            fontSize: "1.2rem",
            color: display >= star ? "#f5a623" : "#ccc",
          }}
        >
          ★
        </span>
      ))}
    </div>
  );
}
