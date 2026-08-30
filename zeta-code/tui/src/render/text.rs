use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Wrap;

pub(crate) fn line_to_borrowed<'a>(line: &'a Line<'_>) -> Line<'a> {
    Line {
        style: line.style,
        alignment: line.alignment,
        spans: line
            .spans
            .iter()
            .map(|span| Span::styled(span.content.as_ref() as &str, span.style))
            .collect(),
    }
}

pub(crate) fn line_to_static(line: &Line<'_>) -> Line<'static> {
    Line {
        style: line.style,
        alignment: line.alignment,
        spans: line
            .spans
            .iter()
            .map(|span| Span::styled(span.content.to_string(), span.style))
            .collect(),
    }
}

pub(crate) fn push_owned_lines(source: &[Line<'_>], output: &mut Vec<Line<'static>>) {
    output.extend(source.iter().map(line_to_static));
}

pub(crate) fn prefix_lines<'a>(
    lines: Vec<Line<'a>>,
    initial: Span<'a>,
    subsequent: Span<'a>,
) -> Vec<Line<'a>> {
    lines
        .into_iter()
        .enumerate()
        .map(|(index, mut line)| {
            line.spans.insert(
                0,
                if index == 0 {
                    initial.clone()
                } else {
                    subsequent.clone()
                },
            );
            line
        })
        .collect()
}

pub(crate) fn styled_text_lines<'a>(text: &'a str, style: Style) -> Vec<Line<'a>> {
    text.split('\n')
        .map(|line| line.strip_suffix('\r').unwrap_or(line))
        .map(|line| Line::from(Span::styled(line, style)))
        .collect()
}

pub(crate) fn wrapped_height(lines: &[Line<'_>], width: u16) -> usize {
    Paragraph::new(lines.to_vec())
        .wrap(Wrap { trim: false })
        .line_count(width)
}

#[cfg(test)]
#[path = "text_tests.rs"]
mod tests;
