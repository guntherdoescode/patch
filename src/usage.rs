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
    fn unknown_context_is_not_guessed() {
        let counts = TokenCounts { input: 10, output: 5, ..Default::default() };
        assert_eq!(counts.context_fraction(), None);
    }
}
