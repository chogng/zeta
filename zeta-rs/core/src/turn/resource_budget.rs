use crate::{CoreError, TurnSnapshot};
use zeta_protocol::{ModelRef, ModelUsageSummary, TurnResourceBudget};

const TOKENS_PER_PRICE_UNIT: u128 = 1_000_000;

pub(crate) fn validate_resource_budget(
    model: Option<&ModelRef>,
    budget: Option<&TurnResourceBudget>,
) -> Result<(), CoreError> {
    let Some(budget) = budget else {
        return Ok(());
    };
    if budget.max_total_tokens.is_none() && budget.max_cost_usd_micros.is_none() {
        return Err(CoreError::InvalidInput(
            "Turn resource budget must define a token or cost ceiling".into(),
        ));
    }
    if budget.max_total_tokens == Some(0) || budget.max_cost_usd_micros == Some(0) {
        return Err(CoreError::InvalidInput(
            "Turn resource budget ceilings must be greater than zero".into(),
        ));
    }
    match (budget.max_cost_usd_micros, budget.price_snapshot.as_ref()) {
        (Some(_), Some(price)) => {
            if price.revision.trim().is_empty() {
                return Err(CoreError::InvalidInput(
                    "Turn model price snapshot revision must not be empty".into(),
                ));
            }
            if model != Some(&price.model) {
                return Err(CoreError::InvalidInput(
                    "Turn model price snapshot must match the selected model".into(),
                ));
            }
        }
        (Some(_), None) => {
            return Err(CoreError::InvalidInput(
                "Turn cost budget requires a model price snapshot".into(),
            ));
        }
        (None, Some(_)) => {
            return Err(CoreError::InvalidInput(
                "Turn model price snapshot requires a cost ceiling".into(),
            ));
        }
        (None, None) => {}
    }
    Ok(())
}

pub(crate) fn ensure_resource_budget_available(turn: &TurnSnapshot) -> Result<(), CoreError> {
    let Some(budget) = turn.resource_budget.as_ref() else {
        return Ok(());
    };
    if resource_budget_is_exhausted(&turn.usage, budget) {
        return Err(CoreError::TurnBudgetExhausted);
    }
    Ok(())
}

fn resource_budget_is_exhausted(usage: &ModelUsageSummary, budget: &TurnResourceBudget) -> bool {
    if budget
        .max_total_tokens
        .is_some_and(|limit| reported_total_tokens(usage) >= u128::from(limit))
    {
        return true;
    }
    if let (Some(limit), Some(price)) = (budget.max_cost_usd_micros, &budget.price_snapshot) {
        return reported_cost_numerator(usage, price) >= u128::from(limit) * TOKENS_PER_PRICE_UNIT;
    }
    false
}

fn reported_total_tokens(usage: &ModelUsageSummary) -> u128 {
    let input = usage
        .input_tokens
        .reported
        .max(usage.cached_input_tokens.reported);
    u128::from(input) + u128::from(usage.output_tokens.reported)
}

fn reported_cost_numerator(
    usage: &ModelUsageSummary,
    price: &zeta_protocol::ModelPriceSnapshot,
) -> u128 {
    let input = u128::from(
        usage
            .input_tokens
            .reported
            .max(usage.cached_input_tokens.reported),
    );
    let output = u128::from(usage.output_tokens.reported);
    let input_cost = if usage.input_tokens.complete && usage.cached_input_tokens.complete {
        let cached = input.min(u128::from(usage.cached_input_tokens.reported));
        let uncached = input - cached;
        checked_token_cost(cached, price.cached_input_usd_micros_per_million_tokens)
            .checked_add(checked_token_cost(
                uncached,
                price.input_usd_micros_per_million_tokens,
            ))
            .unwrap_or(u128::MAX)
    } else {
        checked_token_cost(
            input,
            price
                .input_usd_micros_per_million_tokens
                .min(price.cached_input_usd_micros_per_million_tokens),
        )
    };
    input_cost
        .checked_add(checked_token_cost(
            output,
            price.output_usd_micros_per_million_tokens,
        ))
        .unwrap_or(u128::MAX)
}

fn checked_token_cost(tokens: u128, rate: u64) -> u128 {
    tokens.checked_mul(u128::from(rate)).unwrap_or(u128::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use zeta_protocol::{ModelId, ModelPriceSnapshot, ModelUsage, ProviderId};

    fn model() -> ModelRef {
        ModelRef::new(
            ProviderId::new("provider").expect("valid provider"),
            ModelId::new("model").expect("valid model"),
        )
    }

    fn prices() -> ModelPriceSnapshot {
        ModelPriceSnapshot {
            model: model(),
            revision: "prices-2026-08-23".into(),
            input_usd_micros_per_million_tokens: 10,
            cached_input_usd_micros_per_million_tokens: 2,
            output_usd_micros_per_million_tokens: 20,
        }
    }

    #[test]
    fn cost_lower_bound_uses_exact_cached_split_only_when_complete() {
        let complete = ModelUsageSummary::default()
            .checked_record(Some(&ModelUsage {
                input_tokens: Some(100),
                output_tokens: Some(10),
                cached_input_tokens: Some(40),
                reasoning_tokens: None,
            }))
            .expect("usage fits");
        assert_eq!(reported_cost_numerator(&complete, &prices()), 880);

        let incomplete = complete
            .checked_record(Some(&ModelUsage {
                input_tokens: None,
                output_tokens: Some(0),
                cached_input_tokens: Some(0),
                reasoning_tokens: None,
            }))
            .expect("usage fits");
        assert_eq!(reported_cost_numerator(&incomplete, &prices()), 400);

        let cached_only = ModelUsageSummary::default()
            .checked_record(Some(&ModelUsage {
                input_tokens: None,
                output_tokens: Some(0),
                cached_input_tokens: Some(100),
                reasoning_tokens: None,
            }))
            .expect("usage fits");
        assert_eq!(reported_total_tokens(&cached_only), 100);
        assert_eq!(reported_cost_numerator(&cached_only, &prices()), 200);
    }

    #[test]
    fn cost_budget_requires_a_matching_versioned_price_snapshot() {
        assert!(
            validate_resource_budget(Some(&model()), Some(&TurnResourceBudget::default())).is_err()
        );
        assert!(
            validate_resource_budget(
                Some(&model()),
                Some(&TurnResourceBudget {
                    max_total_tokens: Some(0),
                    max_cost_usd_micros: None,
                    price_snapshot: None,
                })
            )
            .is_err()
        );
        let budget = TurnResourceBudget {
            max_total_tokens: None,
            max_cost_usd_micros: Some(10),
            price_snapshot: None,
        };
        assert!(validate_resource_budget(Some(&model()), Some(&budget)).is_err());

        let mut budget = TurnResourceBudget {
            price_snapshot: Some(prices()),
            ..budget
        };
        assert!(validate_resource_budget(Some(&model()), Some(&budget)).is_ok());
        assert!(validate_resource_budget(None, Some(&budget)).is_err());
        budget
            .price_snapshot
            .as_mut()
            .expect("price snapshot")
            .revision = " ".into();
        assert!(validate_resource_budget(Some(&model()), Some(&budget)).is_err());
    }

    #[test]
    fn token_and_cost_limits_are_exhausted_at_the_exact_boundary() {
        let usage = ModelUsageSummary::default()
            .checked_record(Some(&ModelUsage {
                input_tokens: Some(1_000_000),
                output_tokens: Some(0),
                cached_input_tokens: Some(0),
                reasoning_tokens: Some(0),
            }))
            .expect("usage fits");
        assert!(resource_budget_is_exhausted(
            &usage,
            &TurnResourceBudget {
                max_total_tokens: Some(1_000_000),
                max_cost_usd_micros: None,
                price_snapshot: None,
            }
        ));
        assert!(resource_budget_is_exhausted(
            &usage,
            &TurnResourceBudget {
                max_total_tokens: None,
                max_cost_usd_micros: Some(10),
                price_snapshot: Some(prices()),
            }
        ));
    }
}
