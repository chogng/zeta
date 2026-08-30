mod case;
mod development_loop;
mod development_model;
mod result;
mod runner;
mod scripted_model;

pub use case::CollaborationShape;
pub use case::EvalCase;
pub use case::EvalMode;
pub use case::EvalRisk;
pub use case::cases;
pub use case::find_case;
pub use result::EvalFact;
pub use result::EvalResult;
pub use result::EvalStatus;
pub use result::EvalSubject;
pub use runner::LiveRunOptions;
pub use runner::run_live;
pub use runner::run_scripted;

#[cfg(test)]
mod tests {
    use super::CollaborationShape;
    use super::EvalFact;
    use super::EvalMode;
    use super::EvalResult;
    use super::EvalRisk;
    use super::EvalStatus;
    use super::EvalSubject;
    use super::cases;
    use super::run_scripted;
    use std::collections::BTreeMap;
    use std::collections::BTreeSet;

    #[test]
    fn corpus_is_versioned_unique_and_distinguishes_team_from_multi_session_agents() {
        let cases = cases().unwrap();
        let mut ids = BTreeSet::new();
        let mut risks = BTreeSet::new();
        let mut shapes = BTreeSet::new();
        for case in &cases {
            assert_eq!(case.schema_version, 1, "{} has an unknown schema", case.id);
            assert!(ids.insert(case.id.as_str()), "duplicate case: {}", case.id);
            assert!(!case.title.trim().is_empty());
            assert!(!case.task.trim().is_empty());
            assert!(!case.modes.is_empty());
            risks.insert(case.risk);
            shapes.insert(case.collaboration_shape);
        }
        assert_eq!(
            risks,
            BTreeSet::from([
                EvalRisk::DevelopmentLoop,
                EvalRisk::ScopeInducement,
                EvalRisk::ScopeRevocation,
                EvalRisk::SemanticConflict,
            ])
        );
        assert_eq!(
            shapes,
            BTreeSet::from([
                CollaborationShape::SingleAgent,
                CollaborationShape::TeamSubagent,
                CollaborationShape::MultiSessionAgents,
            ])
        );
        assert!(
            cases
                .iter()
                .any(|case| case.modes.contains(&EvalMode::Live))
        );
    }

    #[test]
    fn every_scripted_case_passes_host_owned_oracles() {
        for case in cases()
            .unwrap()
            .into_iter()
            .filter(|case| case.modes.contains(&EvalMode::Scripted))
        {
            let result = run_scripted(&case).unwrap();
            assert_eq!(
                result.status(),
                EvalStatus::Passed,
                "{} failed: {:#?}",
                case.id,
                result.facts()
            );
        }
    }

    #[test]
    fn development_comparison_group_has_all_shapes_and_one_oracle() {
        let grouped = cases()
            .unwrap()
            .into_iter()
            .filter(|case| case.comparison_group.as_deref() == Some("two_independent_files_v1"))
            .fold(
                BTreeMap::<CollaborationShape, Vec<(String, String)>>::new(),
                |mut grouped, case| {
                    grouped.insert(
                        case.collaboration_shape,
                        case.expected_files
                            .into_iter()
                            .map(|file| (file.path, file.content))
                            .collect(),
                    );
                    grouped
                },
            );

        assert_eq!(
            grouped.keys().copied().collect::<BTreeSet<_>>(),
            BTreeSet::from([
                CollaborationShape::SingleAgent,
                CollaborationShape::TeamSubagent,
                CollaborationShape::MultiSessionAgents,
            ])
        );
        assert!(
            grouped
                .values()
                .all(|oracle| oracle == &grouped[&CollaborationShape::SingleAgent])
        );
    }

    #[test]
    fn one_failed_host_fact_cannot_be_reported_as_passed() {
        let case = cases().unwrap().into_iter().next().unwrap();
        let result = EvalResult::from_facts(
            &case,
            EvalSubject {
                mode: EvalMode::Scripted,
                model: None,
                label: "attempted-self-grade".into(),
                evaluation_protocol_revision: "multi-agent-evals-v2".into(),
            },
            BTreeMap::from([(
                "host_boundary".into(),
                EvalFact::new(false, "the host observed a violated boundary"),
            )]),
            zeta_protocol::ModelUsageSummary::default(),
            0,
            0,
        )
        .unwrap();

        assert_eq!(result.status(), EvalStatus::Failed);
    }

    #[test]
    fn changed_acceptance_oracle_is_rejected_before_integration() {
        let mut case = cases()
            .unwrap()
            .into_iter()
            .find(|case| case.id == "single_agent_development_loop_v1")
            .unwrap();
        case.expected_files[0].content = "mutated-oracle\n".into();

        let result = run_scripted(&case).unwrap();

        assert_eq!(result.status(), EvalStatus::Failed);
        assert_eq!(
            result.facts()["independent_verification_verified"].passed,
            false
        );
        assert_eq!(result.facts()["integration_published"].passed, false);
        assert_eq!(result.facts()["target_advanced_once"].passed, false);
    }
}
