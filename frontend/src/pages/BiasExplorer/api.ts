// Bias Explorer — API entry point.
//
// Thin re-export of the service functions from services/bias.ts so the
// page module does not depend on the services-layer path layout. If the
// service location ever moves, only this file changes.

export { getAvailableFilters, runBiasQuery } from "../../services/bias";
