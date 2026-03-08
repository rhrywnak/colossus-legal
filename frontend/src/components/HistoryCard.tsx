import { useState } from "react";
import { QAEntrySummary } from "../services/qa";
import { StarRating } from "./StarRating";

interface Props {
  entry: QAEntrySummary;
  onClick: (entryId: string) => void;
  onRate: (rating: number) => void;
}

export function HistoryCard({ entry, onClick, onRate }: Props) {
  const [currentRating, setCurrentRating] = useState<number | null>(
    entry.user_rating
  );
  const [isHovered, setIsHovered] = useState(false);

  const handleRate = (rating: number) => {
    setCurrentRating(rating);
    onRate(rating);
  };

  const formattedDate = new Date(entry.asked_at).toLocaleString();
  const durationSec = entry.total_ms != null
    ? `${(entry.total_ms / 1000).toFixed(1)}s`
    : null;

  return (
    <div
      onClick={() => onClick(entry.id)}
      onMouseEnter={() => setIsHovered(true)}
      onMouseLeave={() => setIsHovered(false)}
      style={{
        border: `1px solid ${isHovered ? "#999" : "#e0e0e0"}`,
        borderRadius: "6px",
        padding: "0.75rem 1rem",
        marginBottom: "0.75rem",
        cursor: "pointer",
        backgroundColor: isHovered ? "#fafafa" : "transparent",
      }}
    >
      <div style={{ fontWeight: 500, marginBottom: "0.4rem" }}>
        {entry.question_preview}
      </div>
      <div style={{ fontSize: "0.8rem", color: "#666", marginBottom: "0.4rem" }}>
        <span>{entry.asked_by}</span>
        <span> · </span>
        <span>{formattedDate}</span>
        {durationSec && (
          <>
            <span> · </span>
            <span>{durationSec}</span>
          </>
        )}
        <span> · </span>
        <span>{entry.model}</span>
      </div>
      <StarRating value={currentRating} onChange={handleRate} />
    </div>
  );
}
