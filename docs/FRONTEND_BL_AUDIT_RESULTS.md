# Frontend Business Logic Audit Results

**Date:** 2026-04-06
**Branch:** main (beta.48)
**Auditor:** Claude Code

---

## HIGH Severity (drives pipeline flow or access control)

### Finding F-1
**File:** `pages/DocumentWorkspaceTabs.tsx` (line 91, 95-106)
**Pattern:** Status string comparison + conditional action rendering
**Code:**
```tsx
const isPublished = doc?.status === "PUBLISHED";
// ...
return ALL_TABS.filter((tab) => {
  switch (tab.id) {
    case "content": return isPublished || isAdmin;
    case "processing": return isAdmin;
    case "review": return isAdmin || isAssignedReviewer;
    case "people": return isPublished || isAdmin;
  }
});
```
**Violation:** Yes
**Severity:** HIGH
**What should happen:** The backend should return `visible_tabs: ["document", "content", "processing", "review", "people"]` based on user role and document status. The frontend should render only the tabs the backend says are visible.

---

### Finding F-2
**File:** `components/documents/DocumentCard.tsx` (line 57-58)
**Pattern:** Status string comparison driving interactivity
**Code:**
```tsx
const isPublished = doc.status === "PUBLISHED";
const canInteract = isAdmin || isPublished;
```
**Violation:** Yes
**Severity:** HIGH
**What should happen:** The backend document list response should include `can_view: true/false` per document. The frontend should use this flag for opacity/pointer-events instead of computing it from status + role.

---

### Finding F-3
**File:** `components/pipeline/ReviewPanel.tsx` (line 248)
**Pattern:** Status string comparison controlling action availability
**Code:**
```tsx
{(!item.review_status || item.review_status.toLowerCase() === "pending") && (
  // renders approve/reject/edit buttons
)}
```
**Violation:** Yes
**Severity:** HIGH
**What should happen:** Each item from the backend should include `can_approve: boolean`, `can_reject: boolean`, `can_edit: boolean` flags. The frontend renders buttons based on those flags, not by interpreting review_status.

---

## MEDIUM Severity (drives display based on client-side logic)

### Finding F-4
**File:** `pages/DocumentsPage.tsx` (lines 17-22)
**Pattern:** Status string comparison for display grouping
**Code:**
```tsx
function statusBucket(status: string): string {
  if (status === "PUBLISHED") return "published";
  if (status === "VERIFIED" || status === "REVIEWED") return "in_review";
  if (status === "UPLOADED") return "uploaded";
  return "processing";
}
```
**Violation:** Yes
**Severity:** MEDIUM
**What should happen:** The backend document list should include `status_group: "published" | "processing" | "in_review" | "uploaded"` so the frontend doesn't need to know the status-to-group mapping.

### Finding F-5
**File:** `components/documents/BatchProgressHeader.tsx` (lines 30-35)
**Pattern:** Identical statusBucket function duplicated
**Code:**
```tsx
function statusBucket(status: string): string {
  if (status === "PUBLISHED") return "published";
  if (status === "UPLOADED") return "uploaded";
  if (status === "VERIFIED" || status === "REVIEWED") return "in_review";
  return "processing";
}
```
**Violation:** Yes
**Severity:** MEDIUM
**What should happen:** Same as F-4. Additionally, this is duplicated code — the same function exists in both files.

### Finding F-6
**File:** `components/pipeline/ReviewPanel.tsx` (lines 100-102)
**Pattern:** Client-side counting/aggregation for summary display
**Code:**
```tsx
const pending = items.filter((it) => !it.review_status || it.review_status.toLowerCase() === "pending").length;
const approved = items.filter((it) => it.review_status?.toLowerCase() === "approved").length;
const rejected = items.filter((it) => it.review_status?.toLowerCase() === "rejected").length;
```
**Violation:** Yes
**Severity:** MEDIUM
**What should happen:** The backend review endpoint response should include `summary: { pending: N, approved: N, rejected: N }` counts. The frontend should display these server-computed values instead of counting client-side.

### Finding F-7
**File:** `pages/EvidenceExplorerPage.tsx` (lines 48, 60)
**Pattern:** Client-side aggregation for summary display
**Code:**
```tsx
provenCount: sorted.filter((a) => a.evidence_status?.toUpperCase() === "PROVEN").length,
// ... repeated for "other" group
provenCount: other.filter((a) => a.evidence_status?.toUpperCase() === "PROVEN").length,
```
**Violation:** Yes
**Severity:** MEDIUM
**What should happen:** The allegations API response should include `proven_count` per legal count group, computed by the backend. The frontend shouldn't filter and count by status string.

### Finding F-8
**File:** `components/pipeline/PeopleLinksPanel.tsx` (lines 17, 50, 62-63)
**Pattern:** Entity type string comparison for data grouping
**Code:**
```tsx
const PEOPLE_TYPES = new Set(["Person", "Organization"]);
// ...
if (!PEOPLE_TYPES.has(item.entity_type)) continue;
// ...
people: all.filter((e) => e.entityType === "Person"),
orgs: all.filter((e) => e.entityType === "Organization"),
```
**Violation:** Yes
**Severity:** MEDIUM
**What should happen:** The backend should provide a people/organizations endpoint that returns pre-grouped data: `{ people: [...], organizations: [...] }`. The frontend shouldn't need to know which entity types are "people types".

### Finding F-9
**File:** `utils/processingSteps.ts` (lines 5-17)
**Pattern:** Hardcoded pipeline step names and order
**Code:**
```tsx
export const STEP_DISPLAY_NAMES: Record<string, string> = {
  upload: "Upload",
  extract_text: "Read Document",
  // ... 8 hardcoded step names
};
export const STEP_ORDER = Object.keys(STEP_DISPLAY_NAMES);
```
**Violation:** Yes
**Severity:** MEDIUM
**What should happen:** The backend state machine already returns `pipeline_stages` with `name`, `label`, and `order` fields. The frontend should use those labels instead of maintaining its own display name mapping. `STEP_ORDER` should come from the backend response order.

### Finding F-10
**File:** `components/pipeline/ProcessingPanel.tsx` (lines 18-25)
**Pattern:** Hardcoded pipeline step-to-API function mapping
**Code:**
```tsx
const TRIGGER_MAP: Record<string, (id: string) => Promise<unknown>> = {
  extract_text: triggerExtractText,
  extract: triggerExtract,
  verify: triggerVerify,
  ingest: triggerIngest,
  index: triggerIndex,
  completeness: fetchCompleteness,
};
```
**Violation:** Yes (partial)
**Severity:** MEDIUM
**What should happen:** The backend already provides `action.action` and `action.method` on each stage. The frontend could use a single generic trigger function: `POST /api/admin/pipeline/documents/:id/:action` instead of individual trigger functions. The TRIGGER_MAP would become unnecessary.

---

## LOW Severity (cosmetic, display-only string comparisons)

### Finding F-11
**File:** `components/pipeline/ProcessingPanel.tsx` (lines 160, 165, 169)
**Pattern:** Status string comparison for styling
**Code:**
```tsx
color: stage.status === "pending" ? "#94a3b8" : "#0f172a",
{stage.status === "completed" ? formatDuration(stage.duration_secs) : "\u2014"}
color: stage.status === "failed" ? "#ef4444" : "#64748b",
```
**Violation:** No (borderline)
**Severity:** LOW
**Rationale:** These compare `stage.status` which is provided by the backend state machine. The frontend is rendering based on backend-provided status values. This is acceptable — the backend controls what status each stage has. However, the color mapping could be moved to `useSchema` or be provided by the backend.

### Finding F-12
**File:** `components/pipeline/ExecutionHistory.tsx` (lines 48, 56)
**Pattern:** Status string comparison for styling
**Code:**
```tsx
color: s.status === "completed" ? "#22c55e" : s.status === "failed" ? "#ef4444" : "#2563eb",
{s.status === "failed" && s.error_message && (
```
**Violation:** No
**Severity:** LOW
**Rationale:** Styling based on backend-provided status values. The backend controls the status; the frontend just picks a color. Acceptable pattern.

### Finding F-13
**File:** `utils/nodeTypeDisplay.ts` (line 62)
**Pattern:** Entity type string comparison for formatting
**Code:**
```tsx
if (nodeType === "ComplaintAllegation") return `¶${pageNumber}`;
```
**Violation:** No (borderline)
**Severity:** LOW
**Rationale:** This is a formatting decision — allegations use paragraph marks (¶) vs page numbers (p.). While it's an entity-type-specific display rule, it's purely cosmetic. The backend could provide a `page_label_format: "paragraph" | "page"` field on each entity type, but this is low priority.

### Finding F-14
**File:** `pages/DocumentWorkspace.tsx` (line 62)
**Pattern:** Entity type string comparison for navigation behavior
**Code:**
```tsx
if (ev.node_type === "ComplaintAllegation") return; // paragraph numbers, not PDF pages
```
**Violation:** No (borderline)
**Severity:** LOW
**Rationale:** Same paragraph-vs-page distinction as F-13. Prevents navigating to a paragraph number as if it were a page number. The backend could include `has_page_reference: boolean` on each item.

### Finding F-15
**File:** `components/pipeline/ContentPanel.tsx` (line 86)
**Pattern:** Grounding status string comparison for badge style
**Code:**
```tsx
<span style={groundBadge(item.grounding_status === "grounded")}>
```
**Violation:** No (borderline)
**Severity:** LOW
**Rationale:** Simple boolean styling — green if grounded, yellow if not. The backend could return `is_grounded: boolean` but this is a cosmetic rendering decision.

### Finding F-16
**File:** `components/pipeline/ReviewPanel.tsx` (lines 83-91)
**Pattern:** Client-side filtering on review_status and grounding_status
**Code:**
```tsx
return items.filter((it) => {
  if (reviewFilter !== "all" && (it.review_status || "pending") !== reviewFilter) return false;
  if (groundFilter !== "all" && (it.grounding_status || "unknown") !== groundFilter) return false;
  return true;
});
```
**Violation:** No
**Severity:** LOW
**Rationale:** This is client-side filtering of already-displayed data in a dropdown/select control. The backend already provides the full item list with all statuses. Filtering within a UI view for the user's convenience is acceptable frontend behavior.

---

## False Positives Skipped

- `DeleteConfirmDialog.tsx` `canDelete` — form validation (user must fill reason + match title). This is UX validation, not business logic.
- `SearchPage.tsx` `getDetailLink()` — navigation routing based on node_type. This is routing logic (which page to link to), not business logic.
- `SearchPage.tsx` `activeTypes.filter()` — UI filter chip toggle state.
- `ContentPanel.tsx` `items.filter(entity_type)` — client-side filter of displayed data in a dropdown.
- `AllegationsPage.tsx` `allegations.filter()` — client-side search/filter on displayed list.
- `TimelinePage.tsx` `data.events.filter(category)` — client-side category filter.
- `AskPage.tsx` `prev.filter()` — removing item from UI history list.
- `RetrievalDetailsPanel.tsx` `.filter(d => d.origin)` — splitting display into qdrant/graph sections.
- `AdminMetrics.tsx` `.filter()` — rendering existing data.
- `AdminDocuments.tsx` `.reduce()` — summing counts for display (v1 dead code anyway).

---

## Summary

- **Total findings:** 16
- **HIGH:** 3 (drives pipeline flow or access control)
- **MEDIUM:** 7 (drives display based on client-side logic)
- **LOW:** 6 (cosmetic/borderline, no violations)
- **False positives skipped:** 10

## Recommended Fix Order

1. **F-1** (DocumentWorkspaceTabs tab visibility) — Access control decision made client-side. Backend should return `visible_tabs` array.
2. **F-2** (DocumentCard canInteract) — Access control decision made client-side. Backend should return `can_view` per document.
3. **F-3** (ReviewPanel action buttons) — Business logic deciding which actions are available. Backend should return `can_approve/can_reject/can_edit` per item.
4. **F-6** (ReviewPanel summary counts) — Client-side aggregation of review counts. Backend should include summary counts in the response.
5. **F-4 + F-5** (statusBucket duplication) — Backend should return `status_group` on each document. Eliminates duplicated mapping logic.
6. **F-9 + F-10** (processingSteps + TRIGGER_MAP) — Backend already provides stage labels and actions. Frontend should use them instead of maintaining its own mapping.
7. **F-8** (PeopleLinksPanel entity type grouping) — Backend should return pre-grouped people data.
8. **F-7** (EvidenceExplorer proven counts) — Backend should include proven_count in the response.
