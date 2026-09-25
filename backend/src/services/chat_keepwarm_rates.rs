//! What a pre-warm costs, in dollars, for the one model whose prices are known.
//!
//! ## DEBT(CC_TASK_COST_PAGE_v1): prices in code
//!
//! Standing Rule 2 says prices belong in the model registry (`llm_models`), and
//! CC_TASK_COST_PAGE_v1 moves them there. Until then the three prices the Admin
//! "Chat case file" box needs live here as named constants, each from
//! Anthropic's pricing page (<https://platform.claude.com/docs/en/about-claude/pricing>,
//! "Model pricing" table, row "Claude Opus 5.5", fetched 2026-09-24). Ruled
//! 2026-09-24 in CC_TASK_KEEPWARM_BUTTON_v1, Law 10. For ANY other model these
//! functions answer `None` and the page says the cost is not known, rather than
//! price one model at another's rates.

use colossus_chat::{CacheTtl, Usage};

/// DEBT(CC_TASK_COST_PAGE_v1): the only model these prices belong to.
pub const PRICED_MODEL: &str = "claude-opus-5-5";
/// DEBT(CC_TASK_COST_PAGE_v1): "Base input tokens $4 / MTok".
pub const INPUT_USD_PER_TOKEN: f64 = 4.0e-6;
/// DEBT(CC_TASK_COST_PAGE_v1): "Cache hits and refreshes $0.20 / MTok".
pub const CACHE_READ_USD_PER_TOKEN: f64 = 0.20e-6;
/// DEBT(CC_TASK_COST_PAGE_v1): "1h cache writes $8 / MTok".
pub const CACHE_WRITE_1H_USD_PER_TOKEN: f64 = 8.0e-6;
/// DEBT(CC_TASK_COST_PAGE_v1): "5m cache writes $5 / MTok".
pub const CACHE_WRITE_5M_USD_PER_TOKEN: f64 = 5.0e-6;

/// The write price for the cache lifetime the chat is set to.
fn write_rate(ttl: CacheTtl) -> f64 {
    match ttl {
        CacheTtl::OneHour => CACHE_WRITE_1H_USD_PER_TOKEN,
        CacheTtl::FiveMinutes => CACHE_WRITE_5M_USD_PER_TOKEN,
    }
}

/// What one pre-warm cost, from its measured usage; `None` for an unpriced model.
///
/// A pre-warm generates nothing (`max_tokens: 0`). If a reply ever reports
/// output tokens anyway, there is no output price here to charge them at, so the
/// answer is `None` (cost not known) rather than a number that leaves them out.
pub fn prewarm_cost(model: &str, ttl: CacheTtl, usage: &Usage) -> Option<f64> {
    if model != PRICED_MODEL || usage.output_tokens.unwrap_or(0) > 0 {
        return None;
    }
    let n = |v: Option<u64>| v.unwrap_or(0) as f64;
    Some(
        n(usage.input_tokens) * INPUT_USD_PER_TOKEN
            + n(usage.cache_read_input_tokens) * CACHE_READ_USD_PER_TOKEN
            + n(usage.cache_creation_input_tokens) * write_rate(ttl),
    )
}

/// Whether this model's prices are known here. Keep-warm refuses to arm for any
/// model that is not: a dollar cap cannot be kept with no price to count by.
pub fn is_priced(model: &str) -> bool {
    model == PRICED_MODEL
}

/// What READING a case file of `prefix_tokens` would cost; `None` when unpriced.
/// The automatic ping's cap projection before any ping has been measured.
pub fn read_cost(model: &str, prefix_tokens: u64) -> Option<f64> {
    is_priced(model).then_some(prefix_tokens as f64 * CACHE_READ_USD_PER_TOKEN)
}

/// What reloading a case file of `prefix_tokens` would cost; `None` when unpriced.
pub fn reload_cost(model: &str, ttl: CacheTtl, prefix_tokens: u64) -> Option<f64> {
    (model == PRICED_MODEL).then(|| prefix_tokens as f64 * write_rate(ttl))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn usage(read: u64, wrote: u64) -> Usage {
        Usage {
            input_tokens: Some(4),
            output_tokens: Some(0),
            cache_read_input_tokens: Some(read),
            cache_creation_input_tokens: Some(wrote),
        }
    }

    /// The Stage P receipt: 4 input + 368,832 read cost $0.0737824.
    #[test]
    fn a_read_costs_what_stage_p_measured() {
        let c = prewarm_cost(PRICED_MODEL, CacheTtl::OneHour, &usage(368_832, 0)).unwrap();
        assert!((c - 0.073_782_4).abs() < 1e-9, "{c}");
    }

    /// A write of the same case file at the one-hour price: about $2.95.
    #[test]
    fn a_write_costs_the_one_hour_price() {
        let c = prewarm_cost(PRICED_MODEL, CacheTtl::OneHour, &usage(0, 368_832)).unwrap();
        assert!((c - (0.000_016 + 2.950_656)).abs() < 1e-9, "{c}");
        let short = prewarm_cost(PRICED_MODEL, CacheTtl::FiveMinutes, &usage(0, 368_832)).unwrap();
        assert!((short - (0.000_016 + 1.844_16)).abs() < 1e-9, "{short}");
    }

    #[test]
    fn any_other_model_is_not_priced() {
        assert_eq!(
            prewarm_cost("claude-opus-5", CacheTtl::OneHour, &usage(1, 0)),
            None
        );
        assert_eq!(
            reload_cost("claude-opus-5", CacheTtl::OneHour, 368_832),
            None
        );
    }

    #[test]
    fn output_it_cannot_price_makes_the_cost_unknown() {
        let mut u = usage(368_832, 0);
        u.output_tokens = Some(3);
        assert_eq!(prewarm_cost(PRICED_MODEL, CacheTtl::OneHour, &u), None);
    }

    #[test]
    fn a_read_is_the_prefix_at_the_read_price_and_unpriced_is_none() {
        let c = read_cost(PRICED_MODEL, 368_832).unwrap();
        assert!((c - 0.073_766_4).abs() < 1e-9, "{c}");
        assert!(is_priced(PRICED_MODEL));
        assert_eq!(read_cost("claude-opus-5", 368_832), None);
    }

    #[test]
    fn a_reload_is_the_prefix_at_the_write_price() {
        let c = reload_cost(PRICED_MODEL, CacheTtl::OneHour, 368_832).unwrap();
        assert!((c - 2.950_656).abs() < 1e-9, "{c}");
    }
}
