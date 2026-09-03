use super::constant_name;
use super::terminal_dimensions;

#[test]
fn default_terminal_dimensions_compensate_for_tall_cells() {
    assert_eq!(terminal_dimensions(16, 16, None, None).unwrap(), (16, 8));
    assert_eq!(
        terminal_dimensions(32, 16, Some(12), None).unwrap(),
        (12, 3)
    );
    assert_eq!(terminal_dimensions(32, 16, None, Some(3)).unwrap(), (12, 3));
}

#[test]
fn rust_constant_names_are_explicit_and_stable() {
    assert_eq!(constant_name("WELCOME_PET").unwrap(), "WELCOME_PET");
    assert!(constant_name("WelcomePet").is_err());
    assert!(constant_name("9PET").is_err());
}
