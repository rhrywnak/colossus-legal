import { API_BASE_URL } from "./api";

export type DocumentItem = {
  id: string;
  title: string;
  docType: string;
  createdAt?: string;
};

export type DocumentDetail = {
  id: string;
  title: string;
  doc_type: string;
  created_at?: string;
  description?: string;
  file_path?: string;
  uploaded_at?: string;
  related_claim_id?: string;
  source_url?: string;
};

export type DocumentUpdateRequest = {
  title?: string;
  doc_type?: string;
  created_at?: string;
};

type ServiceError =
  | { kind: "validation"; field?: string; message: string }
  | { kind: "not_found"; message: string }
  | { kind: "network"; message: string };

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

function mapDocumentDetail(raw: any): DocumentDetail {
  return {
    id: raw?.id ? String(raw.id) : "",
    title: raw?.title ? String(raw.title) : "",
    doc_type: raw?.doc_type ? String(raw.doc_type) : raw?.docType ?? "",
    created_at: raw?.created_at ?? raw?.createdAt ?? undefined,
    description: raw?.description ?? undefined,
    file_path: raw?.file_path ?? raw?.filePath ?? undefined,
    uploaded_at: raw?.uploaded_at ?? raw?.uploadedAt ?? undefined,
    related_claim_id: raw?.related_claim_id ?? raw?.relatedClaimId ?? undefined,
    source_url: raw?.source_url ?? raw?.sourceUrl ?? undefined,
  };
}

async function parseJson(response: Response) {
  try {
    return await response.json();
  } catch (error) {
    throw { kind: "network", message: "Failed to parse response" } as ServiceError;
  }
}

export async function getDocument(id: string): Promise<DocumentDetail> {
  const response = await fetch(`${API_BASE_URL}/documents/${encodeURIComponent(id)}`);

  if (response.status === 404) {
    throw { kind: "not_found", message: "Document not found" } as ServiceError;
  }

  if (!response.ok) {
    throw {
      kind: "network",
      message: `Failed to fetch document: ${response.status}`,
    } as ServiceError;
  }

  const data = await parseJson(response);
  if (!data || typeof data !== "object") {
    throw { kind: "network", message: "Invalid document response shape" } as ServiceError;
  }

  return mapDocumentDetail(data);
}

export async function updateDocument(
  id: string,
  payload: DocumentUpdateRequest
): Promise<DocumentDetail> {
  const response = await fetch(`${API_BASE_URL}/documents/${encodeURIComponent(id)}`, {
    method: "PUT",
    headers: {
      "Content-Type": "application/json",
    },
    body: JSON.stringify(payload),
  });

  if (response.status === 404) {
    throw { kind: "not_found", message: "Document not found" } as ServiceError;
  }

  if (response.status === 400) {
    const data = await parseJson(response);
    const field = data?.details?.field ?? undefined;
    const message = data?.message ?? "Validation error";
    throw { kind: "validation", field, message } as ServiceError;
  }

  if (!response.ok) {
    throw {
      kind: "network",
      message: `Failed to update document: ${response.status}`,
    } as ServiceError;
  }

  const data = await parseJson(response);
  if (!data || typeof data !== "object") {
    throw { kind: "network", message: "Invalid document response shape" } as ServiceError;
  }

  return mapDocumentDetail(data);
}
