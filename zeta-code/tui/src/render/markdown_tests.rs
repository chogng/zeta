use super::*;
use crate::render::test_context;

fn render_text(source: &str, width: usize) -> Vec<HyperlinkLine> {
    let context = test_context();
    let mut output = Vec::new();
    for block in blocks(source) {
        if !output.is_empty() {
            output.push(HyperlinkLine::default());
        }
        output.extend(render(&block, width, context, &mut |_, language, code| {
            crate::render::highlight_code(code, language, context.into())
        }));
    }
    output
}

#[test]
fn markdown_preserves_structure_styles_and_link_targets() {
    let rows = render_text(
        "# Heading\n\n**bold** and *italic* and ~~gone~~ [site](https://example.com)\n\n> quote\n\n- [x] done\n- next\n\n~~~rust\nfn main() {}\n~~~",
        70,
    );
    let text = rows
        .iter()
        .map(|row| row.line.to_string())
        .collect::<Vec<_>>()
        .join("\n");
    insta::assert_snapshot!("markdown_structure", text);
    assert!(
        rows[0]
            .line
            .spans
            .iter()
            .any(|span| span.style.add_modifier.contains(Modifier::BOLD))
    );
    assert!(
        rows.iter()
            .flat_map(|row| &row.links)
            .any(|link| link.destination == "https://example.com/")
    );
    assert!(
        rows.iter()
            .flat_map(|row| &row.line.spans)
            .any(|span| span.style.fg == Some(test_context().keyword()))
    );
}

#[test]
fn tables_reflow_to_records_and_preserve_links() {
    let source = "| Name | Result |\n| :--- | ---: |\n| [中文](https://example.com) | complete |\n| beta | pending |";
    for width in [40, 12] {
        let rows = render_text(source, width);
        assert!(rows.iter().all(|row| row.line.width() <= width));
        assert!(
            rows.iter()
                .flat_map(|row| &row.links)
                .any(|link| link.destination == "https://example.com/")
        );
        let text = rows
            .iter()
            .map(|row| row.line.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        if width == 40 {
            insta::assert_snapshot!("markdown_table", text);
        } else {
            insta::assert_snapshot!("markdown_table_records", text);
        }
    }
}

#[test]
fn local_links_remain_copyable_and_never_become_arbitrary_file_actions() {
    let rows = render_text("[source](src/main.rs) [script](file:///tmp/run.sh)", 80);
    assert!(rows.iter().all(|row| row.links.is_empty()));
    let text = rows
        .iter()
        .map(|row| row.line.to_string())
        .collect::<String>();
    assert!(text.contains("src/main.rs"));
    assert!(text.contains("file:///tmp/run.sh"));
}

#[test]
fn code_blank_lines_and_nested_lists_stay_inside_the_content_width() {
    let rows = render_text(
        "```rust\nfn main() {\n\n    run();\n}\n```\n\n10. first item with a long label\n    - nested item",
        18,
    );
    assert!(rows.iter().all(|row| row.line.width() <= 18), "{rows:?}");
    let text = rows
        .iter()
        .map(|row| row.line.to_string())
        .collect::<Vec<_>>();
    assert!(text.iter().any(|line| line.is_empty()));
    assert!(text.iter().any(|line| line.contains("run();")));
}

#[test]
fn code_link_labels_keep_their_destination_and_list_continuations_align() {
    let rows = render_text(
        "[`docs`](https://example.com/docs)\n\n- one two three four",
        12,
    );
    assert_eq!(rows[0].links[0].destination, "https://example.com/docs");
    assert_eq!(rows[2].line.to_string(), "• one two ");
    assert!(rows[3].line.to_string().starts_with("  three"));
}
