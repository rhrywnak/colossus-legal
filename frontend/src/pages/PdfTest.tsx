/**
 * PdfTest — Temporary test page for verifying PdfViewer works.
 * Remove once the Document Workspace is built.
 */
import React, { useState } from "react";
import PdfViewer from "../components/shared/PdfViewer";
import { API_BASE_URL } from "../services/api";

const TEST_PDF_URL = `${API_BASE_URL}/documents/doc-phillips-discovery-response/file`;

const PdfTest: React.FC = () => {
  const [page, setPage] = useState(1);

  return (
    <div style={{ padding: "2rem 0" }}>
      <h2 style={{ fontSize: "1.1rem", fontWeight: 600, color: "#0f172a", marginBottom: "1rem" }}>
        PDF Viewer Test
      </h2>
      <p style={{ fontSize: "0.82rem", color: "#64748b", marginBottom: "1rem" }}>
        Testing with: <code>{TEST_PDF_URL}</code>
      </p>
      <div style={{ maxWidth: "800px" }}>
        <PdfViewer src={TEST_PDF_URL} page={page} onPageChange={setPage} />
      </div>
    </div>
  );
};

export default PdfTest;
