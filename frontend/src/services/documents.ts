import { API_BASE_URL } from "./api";

export type DocumentItem = {
  id: string;
  title: string;
  docType: string;
  createdAt?: string;
};

const stubDocuments: DocumentItem[] = [
  {
    id: "doc-1",
    title: "Complaint",
    docType: "complaint",
    createdAt: "2024-01-15",
  },
  {
    id: "doc-2",
    title: "Hearing Transcript",
    docType: "transcript",
    createdAt: "2024-02-10",
  },
  {
    id: "doc-3",
    title: "Exhibit A",
    docType: "exhibit",
  },
];

export async function getDocuments(): Promise<DocumentItem[]> {
  const response = await fetch(`${API_BASE_URL}/documents`);

  if (!response.ok) {
    throw new Error(
      `Failed to fetch documents: ${response.status} ${response.statusText}`
    );
  }

  let data: unknown;
  try {
    data = await response.json();
  } catch (error) {
    throw new Error("Failed to parse documents response");
  }

  if (!Array.isArray(data)) {
    throw new Error("Invalid documents response shape: expected an array");
  }

  return data.map((doc: any) => ({
    id: doc?.id ? String(doc.id) : "",
    title: doc?.title ? String(doc.title) : "",
    docType: doc?.doc_type
      ? String(doc.doc_type)
      : doc?.docType
      ? String(doc.docType)
      : "",
    createdAt: doc?.created_at ?? doc?.createdAt ?? undefined,
  }));
}

export async function getDocumentsStub(): Promise<DocumentItem[]> {
  // Simulate async fetch; extend to throw errors if needed for testing states.
  return Promise.resolve(stubDocuments);
}
