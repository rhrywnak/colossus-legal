import React from "react";
import { AskResponse } from "../services/ask";
import MarkdownAnswer from "./MarkdownAnswer";
import ExportButtons from "./ExportButtons";

interface Props {
  response: AskResponse;
}

const AnswerDisplay: React.FC<Props> = ({ response }) => {
  return (
    <div>
      {/* Answer text with markdown rendering */}
      <div style={{
        padding: "1.5rem", backgroundColor: "#ffffff", borderRadius: "8px",
        border: "1px solid #e5e7eb", marginBottom: "1rem",
      }}>
        <MarkdownAnswer content={response.answer} />
        <div style={{ marginTop: "1rem", paddingTop: "0.75rem", borderTop: "1px solid #f3f4f6" }}>
          <ExportButtons markdown={response.answer} question={response.question} />
        </div>
      </div>
    </div>
  );
};

export default AnswerDisplay;
