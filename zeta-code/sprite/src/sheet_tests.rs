use super::compile_sprite_sheet;
use std::fs;

#[test]
fn named_frames_and_actions_compile_without_resampling() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("pet.sprite");
    fs::write(
        &path,
        "version 1\nsize 2 2\ncolor B #4085AC\ncolor K #000000\n\nframe idle\nBB\nBB\nend\n\nframe blink\nKK\nKK\nend\n\naction click\nblink 75\nend\n",
    )
    .unwrap();

    let sheet = compile_sprite_sheet(&path, 128).unwrap();

    assert_eq!(sheet.frames().len(), 2);
    assert_eq!(sheet.source_dimensions(), (2, 2));
    assert_eq!(sheet.frames()[0].name(), "idle");
    assert_eq!(sheet.frames()[0].sprite().as_sprite().width(), 1);
    assert_eq!(sheet.frames()[0].sprite().as_sprite().height(), 1);
    assert_eq!(sheet.idle_frame_index(), 0);
    assert_eq!(sheet.action("click").unwrap().steps().len(), 1);
    assert_eq!(sheet.action("click").unwrap().steps()[0].frame_index(), 1);
    assert_eq!(sheet.action("click").unwrap().steps()[0].duration_ms(), 75);
}

#[test]
fn frames_report_their_name_when_a_terminal_cell_cannot_encode_the_colors() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("pet.sprite");
    fs::write(
        &path,
        "version 1\nsize 2 2\ncolor B #4085AC\ncolor K #000000\n\nframe idle\nBK\n..\nend\n",
    )
    .unwrap();

    let error = compile_sprite_sheet(&path, 128).unwrap_err();

    assert!(error.to_string().contains("compile frame 'idle'"));
    assert!(format!("{error:#}").contains("terminal cell (0, 0)"));
}

#[test]
fn parser_rejects_unknown_action_frames_and_invalid_timing() {
    let directory = tempfile::tempdir().unwrap();
    let unknown = directory.path().join("unknown.sprite");
    fs::write(
        &unknown,
        "version 1\nsize 2 2\ncolor B #4085AC\n\nframe idle\nBB\nBB\nend\n\naction click\nmissing 75\nend\n",
    )
    .unwrap();
    let timing = directory.path().join("timing.sprite");
    fs::write(
        &timing,
        "version 1\nsize 2 2\ncolor B #4085AC\n\nframe idle\nBB\nBB\nend\n\naction click\nidle 80\nend\n",
    )
    .unwrap();

    assert!(
        compile_sprite_sheet(&unknown, 128)
            .unwrap_err()
            .to_string()
            .contains("unknown frame 'missing'")
    );
    assert!(
        compile_sprite_sheet(&timing, 128)
            .unwrap_err()
            .to_string()
            .contains("multiple of 25ms")
    );
}

#[test]
fn actions_return_to_idle_implicitly() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("pet.sprite");
    fs::write(
        &path,
        "version 1\nsize 2 2\ncolor B #4085AC\n\nframe idle\nBB\nBB\nend\n\naction click\nidle 75\nend\n",
    )
    .unwrap();

    let error = compile_sprite_sheet(&path, 128).unwrap_err();

    assert!(error.to_string().contains("must not reference idle"));
}

#[test]
fn every_sheet_requires_an_idle_frame() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("pet.sprite");
    fs::write(
        &path,
        "version 1\nsize 2 2\ncolor B #4085AC\n\nframe wave\nBB\nBB\nend\n",
    )
    .unwrap();

    let error = compile_sprite_sheet(&path, 128).unwrap_err();

    assert!(error.to_string().contains("must define frame 'idle'"));
}
