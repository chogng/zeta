use super::*;

#[test]
fn rejects_an_empty_goal_objective() {
    assert_eq!(
        GoalPromptContext::new("  ", GoalBudget::Unbounded),
        Err(GoalPromptError::EmptyObjective)
    );
}

#[test]
fn renders_and_escapes_a_limited_goal() {
    let context = GoalPromptContext::new(
        "Update <crate> & verify {{ budget }}",
        GoalBudget::Limited {
            token_budget: 100,
            tokens_used: 65,
        },
    )
    .unwrap();

    let prompt = render_goals_prompt(context);

    assert!(
        prompt
            .body()
            .contains("Update &lt;crate&gt; &amp; verify {{ budget }}")
    );
    assert!(prompt.body().contains("token budget: 100"));
    assert!(prompt.body().contains("tokens used: 65"));
    assert!(prompt.body().contains("tokens remaining: 35"));
    assert_eq!(prompt.source(), GOALS_PROMPT);
}

#[test]
fn renders_an_unbounded_goal() {
    let context = GoalPromptContext::new("Finish the migration", GoalBudget::Unbounded).unwrap();

    assert!(
        render_goals_prompt(context)
            .body()
            .contains("mode: unbounded")
    );
}
