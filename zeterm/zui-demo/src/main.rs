fn main() {
    let stats = zui_demo::render_demo().expect("demo renderer should accept the UI scene");
    println!(
        "zui demo rendered {} scene: {} rect, {} icon, {} text",
        stats.scene_count, stats.rect_count, stats.icon_count, stats.text_count
    );
}
