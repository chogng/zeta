use unicode_width::UnicodeWidthStr;

pub(super) fn estimated_wrapped_rows(
    label_width: usize,
    text: &str,
    available_width: usize,
) -> usize {
    if available_width == 0 {
        return 0;
    }
    text.lines()
        .enumerate()
        .map(|(index, line)| {
            let prefix_width = if index == 0 { label_width } else { 0 };
            (prefix_width + line.width())
                .div_ceil(available_width)
                .max(1)
        })
        .sum::<usize>()
        .max(1)
}

#[cfg(test)]
#[path = "row_tests.rs"]
mod tests;
