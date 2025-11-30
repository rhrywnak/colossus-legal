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

export async function getDocumentsStub(): Promise<DocumentItem[]> {
  // Simulate async fetch; extend to throw errors if needed for testing states.
  return Promise.resolve(stubDocuments);
}
