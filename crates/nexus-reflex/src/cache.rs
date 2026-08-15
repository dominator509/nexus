//! Cache accounting (SPEC-009 canonical term CacheLedger; ADR-021).
//!
//! The rolling token cache-hit ratio is hit prompt tokens divided by
//! total prompt tokens; the cacheable reflex traffic target is at least
//! 0.97. Only cacheable requests are recorded in the ledger; the ratio
//! is computed over a fixed rolling window so it reflects recent
//! traffic, never the full history.

use nexus_model_gateway::vocabulary::CacheHitRatio;
use serde::{Deserialize, Serialize};

/// Target cache-hit ratio for cacheable reflex traffic (SPEC-009).
pub const CACHE_TARGET: f64 = 0.97;

/// One recorded cacheable request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CacheRecord {
    pub prompt_tokens: u64,
    pub cache_hit_prompt_tokens: u64,
}

/// Rolling cache ledger.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CacheLedger {
    window: Vec<CacheRecord>,
    max_records: usize,
}

impl CacheLedger {
    pub fn new(max_records: usize) -> Self {
        Self {
            window: Vec::new(),
            max_records: max_records.max(1),
        }
    }

    /// Record a cacheable request's usage. Non-cacheable traffic is
    /// never recorded (callers pass it explicitly).
    pub fn record(&mut self, prompt_tokens: u64, cache_hit_prompt_tokens: u64) {
        if prompt_tokens == 0 {
            return;
        }
        self.window.push(CacheRecord {
            prompt_tokens,
            cache_hit_prompt_tokens: cache_hit_prompt_tokens.min(prompt_tokens),
        });
        if self.window.len() > self.max_records {
            let excess = self.window.len() - self.max_records;
            self.window.drain(0..excess);
        }
    }

    pub fn len(&self) -> usize {
        self.window.len()
    }

    pub fn is_empty(&self) -> bool {
        self.window.is_empty()
    }

    /// Rolling cache-hit ratio over the current window.
    pub fn rolling_ratio(&self) -> CacheHitRatio {
        let hit: u64 = self.window.iter().map(|r| r.cache_hit_prompt_tokens).sum();
        let total: u64 = self.window.iter().map(|r| r.prompt_tokens).sum();
        CacheHitRatio::new(hit, total)
    }

    /// True when the rolling ratio meets the 0.97 cacheable target.
    pub fn meets_cache_target(&self) -> bool {
        self.rolling_ratio().meets_cache_target()
    }
}

impl Default for CacheLedger {
    fn default() -> Self {
        Self::new(128)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep014_unit_empty_ledger_ratio_is_zero() {
        let ledger = CacheLedger::new(8);
        assert!(ledger.is_empty());
        assert_eq!(ledger.rolling_ratio().ratio(), 0.0);
        assert!(!ledger.meets_cache_target());
    }

    #[test]
    fn ep014_unit_rolling_ratio_computes_over_window() {
        let mut ledger = CacheLedger::new(4);
        // 100 prompt tokens, 100 cache hits -> ratio 1.0
        ledger.record(100, 100);
        // 100 prompt tokens, 90 cache hits -> cumulative 190/200 = 0.95
        ledger.record(100, 90);
        let ratio = ledger.rolling_ratio();
        assert_eq!(ratio.hit_tokens(), 190);
        assert_eq!(ratio.total_tokens(), 200);
        assert!((ratio.ratio() - 0.95).abs() < 1e-9);
        assert!(!ledger.meets_cache_target());
    }

    #[test]
    fn ep014_unit_ledger_meets_097_target() {
        let mut ledger = CacheLedger::new(8);
        for _ in 0..8 {
            ledger.record(100, 98);
        }
        assert!(ledger.meets_cache_target());
        assert!(ledger.rolling_ratio().ratio() >= 0.97);
    }

    #[test]
    fn ep014_unit_window_is_bounded() {
        let mut ledger = CacheLedger::new(3);
        for i in 0..10 {
            ledger.record(10, i.min(10));
        }
        assert_eq!(ledger.len(), 3);
    }

    #[test]
    fn ep014_unit_zero_prompt_records_are_ignored() {
        let mut ledger = CacheLedger::new(4);
        ledger.record(0, 0);
        assert!(ledger.is_empty());
    }

    #[test]
    fn ep014_unit_cache_hit_never_exceeds_prompt() {
        let mut ledger = CacheLedger::new(4);
        ledger.record(50, 999);
        assert_eq!(ledger.window[0].cache_hit_prompt_tokens, 50);
    }

    #[test]
    fn ep014_unit_ledger_serde_round_trip() {
        let mut ledger = CacheLedger::new(4);
        ledger.record(100, 98);
        let v = serde_json::to_value(&ledger).unwrap();
        let back: CacheLedger = serde_json::from_value(v).unwrap();
        assert_eq!(back, ledger);
        assert_eq!(back.len(), 1);
    }
}
