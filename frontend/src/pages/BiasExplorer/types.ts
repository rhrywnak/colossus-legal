// Bias Explorer — TypeScript types.
//
// Re-exports the wire shapes from services/bias.ts so the BiasExplorer
// module is self-contained: any sibling component (filters, view,
// EvidenceCard) imports types from here rather than reaching across the
// codebase. When new view types arrive, they import the same shapes.

export type {
    ActorOption,
    AvailableFilters,
    BiasQueryFilters,
    BiasQueryResult,
    BiasInstance,
    DocumentRef,
} from "../../services/bias";
