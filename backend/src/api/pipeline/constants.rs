//! Shared pricing constants for token-based cost estimation.

// NOTE: Documents extracted before this change have cost_usd = NULL in extraction_runs.
// To backfill: UPDATE extraction_runs SET cost_usd =
//   (output_tokens * 0.000015 + input_tokens * 0.000003) WHERE cost_usd IS NULL;

/// Sonnet input cost: $3 per million tokens.
pub const SONNET_INPUT_COST_PER_TOKEN: f64 = 0.000003;

/// Sonnet output cost: $15 per million tokens.
pub const SONNET_OUTPUT_COST_PER_TOKEN: f64 = 0.000015;

/// Estimate cost in USD from token counts using Sonnet pricing.
pub fn estimate_cost(input_tokens: i64, output_tokens: i64) -> f64 {
    (input_tokens as f64 * SONNET_INPUT_COST_PER_TOKEN)
        + (output_tokens as f64 * SONNET_OUTPUT_COST_PER_TOKEN)
}
