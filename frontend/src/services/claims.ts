export type Claim = {
  id: string;
  title: string;
  description?: string;
  status: string;
};

const stubClaims: Claim[] = [
  {
    id: "claim-1",
    title: "Breach of contract",
    description: "Alleged failure to deliver goods as agreed.",
    status: "open",
  },
  {
    id: "claim-2",
    title: "Negligence",
    description: "Failure to exercise reasonable care causing damages.",
    status: "pending",
  },
];

export async function getClaimsStub(): Promise<Claim[]> {
  // Simulate async fetch; can be extended to throw for error-state testing.
  return Promise.resolve(stubClaims);
}
