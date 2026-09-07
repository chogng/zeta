use super::CellView;
use super::ChatHistoryRenderCache;
use super::DetailFormat;
use super::SyntaxHighlighting;
use crate::render::InteractionState;
use crate::render::InteractionTarget;
use crate::render::RenderContext;
use crate::render::interaction_style;
use crate::render::prefix_lines;
use crate::render::push_owned_lines;
use crate::render::styled_text_lines;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use zeta_ansi_escape::ansi_text;

pub(in crate::thread::transcript) fn prefixed_body(
    text: &str,
    marker: &str,
    color: ratatui::style::Color,
    view: &CellView<'_>,
    context: RenderContext<'_>,
    cache: Option<&ChatHistoryRenderCache>,
    highlighting: SyntaxHighlighting,
) -> Vec<Line<'static>> {
    let body = body_lines(
        text,
        selected_style(view.selected, context),
        view.cell_id.as_deref(),
        context,
        cache,
        highlighting,
    );
    let prefixed = prefix_lines(
        body,
        Span::styled(
            format!("{marker} "),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
    );
    let mut lines = Vec::new();
    push_owned_lines(&prefixed, &mut lines);
    lines
}
fn body_lines<'a>(
    text: &'a str,
    style: Style,
    cell_id: Option<&str>,
    context: RenderContext<'_>,
    cache: Option<&ChatHistoryRenderCache>,
    syntax_highlighting: SyntaxHighlighting,
) -> Vec<Line<'a>> {
    if !text
        .lines()
        .any(|line| line.trim_start().starts_with("```"))
    {
        return styled_text_lines(text, style);
    }

    let mut output = Vec::new();
    let mut plain = String::new();
    let mut code = String::new();
    let mut language = String::new();
    let mut in_code = false;
    let mut block_index = 0;
    for source_line in text.split_inclusive('\n') {
        let visible = source_line.strip_suffix('\n').unwrap_or(source_line);
        let visible = visible.strip_suffix('\r').unwrap_or(visible);
        if !in_code {
            if let Some(opening) = visible.trim_start().strip_prefix("```") {
                push_plain_block(&mut output, &mut plain, style);
                language = opening.trim().to_owned();
                in_code = true;
            } else {
                plain.push_str(source_line);
            }
            continue;
        }

        if visible.trim() == "```" {
            push_code_block(
                &mut output,
                cell_id,
                block_index,
                &language,
                &code,
                context,
                cache,
                syntax_highlighting,
            );
            code.clear();
            block_index += 1;
            in_code = false;
        } else {
            code.push_str(source_line);
        }
    }
    if in_code {
        push_code_block(
            &mut output,
            cell_id,
            block_index,
            &language,
            &code,
            context,
            cache,
            syntax_highlighting,
        );
    } else {
        push_plain_block(&mut output, &mut plain, style);
    }
    if output.is_empty() {
        output.push(Line::default());
    }
    output
}

fn push_plain_block(output: &mut Vec<Line<'static>>, text: &mut String, style: Style) {
    if text.is_empty() {
        return;
    }
    let lines = styled_text_lines(text.trim_end_matches('\n'), style);
    push_owned_lines(&lines, output);
    text.clear();
}

#[allow(clippy::too_many_arguments)]
fn push_code_block(
    output: &mut Vec<Line<'static>>,
    cell_id: Option<&str>,
    block_index: usize,
    language: &str,
    code: &str,
    context: RenderContext<'_>,
    cache: Option<&ChatHistoryRenderCache>,
    syntax_highlighting: SyntaxHighlighting,
) {
    if matches!(syntax_highlighting, SyntaxHighlighting::Disabled) {
        let lines = styled_text_lines(
            code.strip_suffix('\n').unwrap_or(code),
            Style::default().fg(context.foreground()),
        );
        push_owned_lines(&lines, output);
        return;
    }
    let lines = cache.map_or_else(
        || crate::render::highlight_code(code, language, context.into()),
        |cache| cache.highlight_code_block(cell_id, block_index, language, code, context),
    );
    output.extend(lines);
}

fn selected_style(selected: bool, context: RenderContext<'_>) -> Style {
    if selected {
        interaction_style(
            context,
            InteractionState {
                target: InteractionTarget::Rest,
                selected: true,
                hovered: false,
                pressed: false,
            },
        )
    } else {
        Style::default()
    }
}

pub(in crate::thread::transcript) fn push_detail_lines(
    lines: &mut Vec<Line<'static>>,
    format: DetailFormat,
    detail: &str,
    context: RenderContext<'_>,
) {
    if matches!(format, DetailFormat::Plain) {
        let detail_lines = prefix_lines(
            styled_text_lines(detail, Style::default().fg(context.muted())),
            Span::styled("└─ ", Style::default().fg(context.muted())),
            Span::raw("   "),
        );
        push_owned_lines(&detail_lines, lines);
        return;
    }

    let mut output = ansi_text(detail).lines;
    if output.is_empty() {
        output.push(Line::default());
    }
    for line in &mut output {
        for span in &mut line.spans {
            if span.style.fg.is_none() {
                span.style.fg = Some(context.muted());
            }
        }
    }
    lines.extend(prefix_lines(
        output,
        Span::styled("└─ ", Style::default().fg(context.muted())),
        Span::raw("   "),
    ));
}
