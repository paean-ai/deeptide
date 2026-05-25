use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ModelPricing {
    pub input: f64,
    pub output: f64,
    pub cache_create: f64,
    pub cache_read: f64,
}

impl ModelPricing {
    pub const fn new(input: f64, output: f64, cache_create: f64, cache_read: f64) -> Self {
        Self {
            input,
            output,
            cache_create,
            cache_read,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TurnRecord {
    pub turn: usize,
    pub model: String,
    pub input_tokens: usize,
    pub output_tokens: usize,
    pub cache_create: usize,
    pub cache_read: usize,
    pub duration_ms: usize,
    pub cost_usd: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CostSummary {
    pub turns: Vec<TurnRecord>,
    pub total_input: usize,
    pub total_output: usize,
    pub total_cache_create: usize,
    pub total_cache_read: usize,
    pub total_cost_usd: f64,
}

impl CostSummary {
    pub fn cache_health(&self) -> CacheHealth {
        self.cache_health_for_recent_turns(3)
    }

    pub fn cache_health_for_recent_turns(&self, recent_turn_count: usize) -> CacheHealth {
        let total = cache_hit_rate(
            self.total_input,
            self.total_cache_create,
            self.total_cache_read,
        );
        let recent_turn_count = recent_turn_count.max(1);
        let recent = self
            .turns
            .iter()
            .rev()
            .take(recent_turn_count)
            .collect::<Vec<_>>();
        let recent_input = recent.iter().map(|turn| turn.input_tokens).sum();
        let recent_create = recent.iter().map(|turn| turn.cache_create).sum();
        let recent_read = recent.iter().map(|turn| turn.cache_read).sum();
        let recent_rate = cache_hit_rate(recent_input, recent_create, recent_read);

        CacheHealth {
            hit_rate_percent: total,
            recent_hit_rate_percent: recent_rate,
            total_create_tokens: self.total_cache_create,
            total_read_tokens: self.total_cache_read,
            turn_count: self.turns.len(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CacheHealth {
    pub hit_rate_percent: Option<usize>,
    pub recent_hit_rate_percent: Option<usize>,
    pub total_create_tokens: usize,
    pub total_read_tokens: usize,
    pub turn_count: usize,
}

impl CacheHealth {
    pub fn label(&self) -> &'static str {
        match self.hit_rate_percent {
            None if self.turn_count <= 1 => "warming",
            None => "unreported",
            Some(rate) if rate >= 80 => "strong",
            Some(rate) if rate >= 60 => "warming",
            Some(_) => "cold",
        }
    }

    pub fn diagnostic(&self) -> Option<&'static str> {
        match self.hit_rate_percent {
            None if self.turn_count <= 1 => {
                Some("first turn usually creates cache before later turns can read it")
            }
            None => Some("provider did not report cache telemetry; verify endpoint support"),
            Some(rate) if rate >= 80 => None,
            Some(_) if self.total_read_tokens == 0 => {
                Some("no cache reads yet; check stable model, base URL, and prompt prefix")
            }
            Some(_) if self.total_create_tokens > self.total_read_tokens => Some(
                "cache is being recreated more than read; avoid changing model, tools, or stable prompt prefix mid-session",
            ),
            Some(_) => Some(
                "cache reads are present but below target; continue the same session to warm the prefix",
            ),
        }
    }
}

#[derive(Debug)]
pub struct CostTracker {
    state: Mutex<CostState>,
    pricing_overrides: HashMap<String, ModelPricing>,
}

impl CostTracker {
    pub fn new() -> Self {
        Self::with_pricing_overrides(HashMap::new())
    }

    pub fn with_pricing_overrides(pricing_overrides: HashMap<String, ModelPricing>) -> Self {
        Self {
            state: Mutex::new(CostState::default()),
            pricing_overrides,
        }
    }

    pub fn record(&self, usage: TurnUsage) {
        let pricing = self.pricing_for(&usage.model);
        let cost_usd = (usage.input_tokens as f64 * pricing.input)
            + (usage.output_tokens as f64 * pricing.output)
            + (usage.cache_create as f64 * pricing.cache_create)
            + (usage.cache_read as f64 * pricing.cache_read);

        let record = TurnRecord {
            turn: usage.turn,
            model: usage.model,
            input_tokens: usage.input_tokens,
            output_tokens: usage.output_tokens,
            cache_create: usage.cache_create,
            cache_read: usage.cache_read,
            duration_ms: usage.duration_ms,
            cost_usd,
        };

        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        state.total_input += record.input_tokens;
        state.total_output += record.output_tokens;
        state.total_cache_create += record.cache_create;
        state.total_cache_read += record.cache_read;
        state.total_cost_usd += record.cost_usd;
        state.turns.push(record);
    }

    pub fn summary(&self) -> CostSummary {
        let state = self
            .state
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        CostSummary {
            turns: state.turns.clone(),
            total_input: state.total_input,
            total_output: state.total_output,
            total_cache_create: state.total_cache_create,
            total_cache_read: state.total_cache_read,
            total_cost_usd: state.total_cost_usd,
        }
    }

    pub fn reset(&self) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        *state = CostState::default();
    }

    pub fn pricing_for(&self, model: &str) -> ModelPricing {
        if let Some(pricing) = self.pricing_overrides.get(model) {
            return *pricing;
        }
        if let Some(pricing) = default_pricing(model) {
            return pricing;
        }
        for (prefix, pricing) in default_pricing_table() {
            if model.starts_with(prefix) {
                return pricing;
            }
        }
        ModelPricing::new(0.0, 0.0, 0.0, 0.0)
    }

    pub fn format_usd(amount: f64) -> String {
        if amount < 0.01 {
            format!("${amount:.4}")
        } else if amount < 1.0 {
            format!("${amount:.3}")
        } else {
            format!("${amount:.2}")
        }
    }

    pub fn format_tokens(count: usize) -> String {
        let digits = count.to_string();
        let mut formatted = String::with_capacity(digits.len() + digits.len() / 3);
        let leading = digits.len() % 3;

        for (index, character) in digits.chars().enumerate() {
            if index > 0 && (index == leading || (index + 3 - leading).is_multiple_of(3)) {
                formatted.push(',');
            }
            formatted.push(character);
        }

        formatted
    }
}

impl Default for CostTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TurnUsage {
    pub turn: usize,
    pub model: String,
    pub input_tokens: usize,
    pub output_tokens: usize,
    pub cache_create: usize,
    pub cache_read: usize,
    pub duration_ms: usize,
}

impl TurnUsage {
    pub fn new(
        turn: usize,
        model: impl Into<String>,
        input_tokens: usize,
        output_tokens: usize,
        cache_create: usize,
        cache_read: usize,
        duration_ms: usize,
    ) -> Self {
        Self {
            turn,
            model: model.into(),
            input_tokens,
            output_tokens,
            cache_create,
            cache_read,
            duration_ms,
        }
    }
}

#[derive(Debug, Default)]
struct CostState {
    turns: Vec<TurnRecord>,
    total_input: usize,
    total_output: usize,
    total_cache_create: usize,
    total_cache_read: usize,
    total_cost_usd: f64,
}

fn cache_hit_rate(input: usize, cache_create: usize, cache_read: usize) -> Option<usize> {
    if cache_create == 0 && cache_read == 0 {
        return None;
    }
    let denominator = (input + cache_create + cache_read).max(1);
    Some((cache_read * 100) / denominator)
}

fn default_pricing(model: &str) -> Option<ModelPricing> {
    default_pricing_table()
        .into_iter()
        .find_map(|(name, pricing)| (name == model).then_some(pricing))
}

fn default_pricing_table() -> [(&'static str, ModelPricing); 5] {
    [
        (
            "deepseek-v4-pro",
            ModelPricing::new(
                0.27 / 1_000_000.0,
                1.10 / 1_000_000.0,
                0.34 / 1_000_000.0,
                0.07 / 1_000_000.0,
            ),
        ),
        (
            "deepseek-v4-flash",
            ModelPricing::new(
                0.07 / 1_000_000.0,
                0.28 / 1_000_000.0,
                0.09 / 1_000_000.0,
                0.02 / 1_000_000.0,
            ),
        ),
        (
            "deepseek-r1",
            ModelPricing::new(
                0.55 / 1_000_000.0,
                2.19 / 1_000_000.0,
                0.69 / 1_000_000.0,
                0.14 / 1_000_000.0,
            ),
        ),
        (
            "deepseek-reasoner",
            ModelPricing::new(
                0.55 / 1_000_000.0,
                2.19 / 1_000_000.0,
                0.69 / 1_000_000.0,
                0.14 / 1_000_000.0,
            ),
        ),
        (
            "deepseek-chat",
            ModelPricing::new(
                0.14 / 1_000_000.0,
                0.28 / 1_000_000.0,
                0.18 / 1_000_000.0,
                0.04 / 1_000_000.0,
            ),
        ),
    ]
}
