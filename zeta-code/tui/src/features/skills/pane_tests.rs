use super::skills_pane_spec;
use crate::components::list_selection::ListSelectionState;
use zeta_app_server_protocol::protocol::skills::{
    SkillDiagnosticCodeDto, SkillDiagnosticDto, SkillListResult,
};

#[test]
fn pane_keeps_backend_diagnostics_out_of_the_tab_model() {
    let diagnostic = SkillDiagnosticDto {
        source: "user:skill-source:personal".into(),
        subject: Some("broken/SKILL.md".into()),
        code: SkillDiagnosticCodeDto::InvalidFrontmatter,
        message: "frontmatter is invalid".into(),
    };
    let catalog = SkillListResult {
        generation: 1,
        skills: Vec::new(),
        diagnostics: vec![diagnostic.clone()],
    };

    let spec = skills_pane_spec(&catalog);
    assert_eq!(spec.diagnostics, vec![diagnostic]);

    let state = ListSelectionState::new(spec.model.into_body());
    assert_eq!(
        state
            .tabs()
            .iter()
            .map(|tab| tab.label())
            .collect::<Vec<_>>(),
        vec!["All (0)", "Enabled (0)", "Disabled (0)", "Manage"]
    );
}
