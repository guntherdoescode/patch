//! Token accounting and transparent cost estimates.
//!
//! Providers must supply usage from their response metadata. Never estimate
//! billing tokens by counting characters when the provider reports usage.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TokenPricing {
    /// USD per one million input tokens.
    pub input_usd_per_million: f64,
    /// USD per one million output tokens.
    pub output_usd_per_million: f64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TokenCounts {
    pub input: u64,
    pub output: u64,
    /// Current prompt/context size, when reported by the provider.
    pub context: Option<u64>,
    /// Model context window, only when obtained from trusted metadata/config.
    pub context_limit: Option<u64>,
}

impl TokenCounts {
    pub fn total_burned(self) -> u64 {
        self.input.saturating_add(self.output)
    }

    pub fn context_fraction(self) -> Option<f64> {
        let used = self.context?;
        let limit = self.context_limit.filter(|limit| *limit > 0)?;
        Some((used as f64 / limit as f64).clamp(0.0, 1.0))
    }

    pub fn estimated_cost_usd(self, pricing: TokenPricing) -> f64 {
        (self.input as f64 * pricing.input_usd_per_million
            + self.output as f64 * pricing.output_usd_per_million)
            / 1_000_000.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CostDisplay {
    Known(f64),
    Unknown,
}

/// Accumulates provider-reported usage for the active conversation.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UsageLedger {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub latest_context_tokens: Option<u64>,
    pub context_limit: Option<u64>,
}

impl UsageLedger {
    /// Record a completed request. The caller must pass provider-reported
    /// usage, not locally guessed token counts.
    pub fn record(
        &mut self,
        input_tokens: u64,
        output_tokens: u64,
        context_tokens: Option<u64>,
        context_limit: Option<u64>,
    ) {
        self.input_tokens = self.input_tokens.saturating_add(input_tokens);
        self.output_tokens = self.output_tokens.saturating_add(output_tokens);
        if context_tokens.is_some() {
            self.latest_context_tokens = context_tokens;
        }
        if context_limit.is_some() {
            self.context_limit = context_limit;
        }
    }

    pub fn counts(&self) -> TokenCounts {
        TokenCounts {
            input: self.input_tokens,
            output: self.output_tokens,
            context: self.latest_context_tokens,
            context_limit: self.context_limit,
        }
    }

    pub fn cost(&self, pricing: Option<TokenPricing>) -> CostDisplay {
        match pricing {
            Some(rates) => CostDisplay::Known(self.counts().estimated_cost_usd(rates)),
            None => CostDisplay::Unknown,
        }
    }
}

pub fn compact_tokens(value: u64) -> String {
    if value >= 1_000_000 {
        format!("{:.2}M", value as f64 / 1_000_000.0)
    } else if value >= 1_000 {
        format!("{:.1}K", value as f64 / 1_000.0)
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn totals_input_and_output_tokens() {
        let counts = TokenCounts { input: 1_250, output: 750, ..Default::default() };
        assert_eq!(counts.total_burned(), 2_000);
    }

    #[test]
    fn computes_separate_input_and_output_rates() {
        let counts = TokenCounts { input: 1_000_000, output: 500_000, ..Default::default() };
        let pricing = TokenPricing {
            input_usd_per_million: 2.0,
            output_usd_per_million: 8.0,
        };
        assert!((counts.estimated_cost_usd(pricing) - 6.0).abs() < f64::EPSILON);
    }

    #[test]
    fn ledger_accumulates_provider_usage_and_tracks_latest_context() {
        let mut ledger = UsageLedger::default();
        ledger.record(100, 25, Some(125), Some(32_768));
        ledger.record(80, 40, Some(245), Some(32_768));
        assert_eq!(ledger.input_tokens, 180);
        assert_eq!(ledger.output_tokens, 65);
        assert_eq!(ledger.latest_context_tokens, Some(245));
        assert_eq!(ledger.counts().total_burned(), 245);
    }

    #[test]
    fn unknown_context_is_not_guessed() {
        let counts = TokenCounts { input: 10, output: 5, ..Default::default() };
        assert_eq!(counts.context_fraction(), None);
    }
}
