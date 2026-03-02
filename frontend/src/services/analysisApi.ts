import { API_BASE_URL } from "./api";
import { authFetch } from "./auth";

// ============================================================================
// Gap Analysis Types
// ============================================================================

export type AllegationStrength = {
  id: string;
  allegation?: string;
  paragraph?: string;
  strength_percent: number;
  strength_category: string; // "strong" | "moderate" | "weak" | "gap"
  supporting_evidence_count: number;
  supporting_evidence?: string[];
  gap_notes?: string;
};

export type GapAnalysis = {
  total_allegations: number;
  strong_evidence: number;
  moderate_evidence: number;
  weak_evidence: number;
  gaps: number;
  allegations: AllegationStrength[];
};

// ============================================================================
// Contradictions Summary Types
// ============================================================================

export type ContradictionBrief = {
  evidence_a_id: string;
  evidence_a_title?: string;
  evidence_a_answer?: string;
  evidence_b_id: string;
  evidence_b_title?: string;
  evidence_b_answer?: string;
  description?: string;
};

export type ContradictionsSummary = {
  total: number;
  contradictions: ContradictionBrief[];
};

// ============================================================================
// Evidence Coverage Types
// ============================================================================

export type DocumentCoverage = {
  document_id: string;
  document_title?: string;
  evidence_count: number;
  linked_count: number;
};

export type EvidenceCoverage = {
  total_evidence_nodes: number;
  linked_to_allegations: number;
  unlinked: number;
  by_document: DocumentCoverage[];
};

// ============================================================================
// Main Response Type
// ============================================================================

export type AnalysisResponse = {
  gap_analysis: GapAnalysis;
  contradictions_summary: ContradictionsSummary;
  evidence_coverage: EvidenceCoverage;
};

// ============================================================================
// API Function
// ============================================================================

export async function getAnalysis(): Promise<AnalysisResponse> {
  const response = await authFetch(`${API_BASE_URL}/analysis`);

  if (!response.ok) {
    throw new Error(`Failed to fetch analysis: ${response.status}`);
  }

  return response.json();
}
