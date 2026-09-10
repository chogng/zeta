use super::ModalLayout;
use ratatui::layout::Rect;

#[test]
fn chrome_remains_inside_resized_terminals() {
    for width in 0..130 {
        for height in 0..45 {
            let available = Rect::new(3, 5, width, height);
            let layout = ModalLayout::new(available, 100, 32);
            assert_eq!(layout.surface.intersection(available), layout.surface);
            for area in [layout.content, layout.footer, layout.title, layout.close] {
                if !area.is_empty() {
                    assert_eq!(area.intersection(layout.surface), area);
                }
            }
            assert!(layout.content.bottom() <= layout.footer.y.max(layout.content.y));
        }
    }
}
