//! Editable pixel-grid parsing for build-time terminal sprites.

use crate::OwnedTerminalSprite;
use crate::pack_quadrants_rgba;
use anyhow::Context;
use anyhow::Result;
use anyhow::bail;
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct SpriteGrid {
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) pixels: Vec<[u8; 4]>,
}

/// Reads an editable `.sprite` grid and packs each 2×2 logical block into a quadrant cell.
pub fn compile_sprite_grid(path: &Path, alpha_threshold: u8) -> Result<OwnedTerminalSprite> {
    let source = read_sprite_grid(path)?;
    pack_quadrants_rgba(source.width, source.height, &source.pixels, alpha_threshold)
        .map_err(anyhow::Error::from)
}

pub(crate) fn read_sprite_grid(path: &Path) -> Result<SpriteGrid> {
    let contents = std::fs::read_to_string(path)
        .with_context(|| format!("read sprite grid {}", path.display()))?;
    parse_sprite_grid(path, &contents)
}

fn parse_sprite_grid(path: &Path, contents: &str) -> Result<SpriteGrid> {
    let mut colors = BTreeMap::new();
    let mut rows = Vec::new();
    let mut reading_grid = false;

    for (index, line) in contents.lines().enumerate() {
        let line_number = index + 1;
        if !reading_grid {
            if line == "---" {
                reading_grid = true;
                continue;
            }
            if line.is_empty() {
                continue;
            }
            let (symbol, color) = parse_palette_entry(line)
                .with_context(|| format!("parse {} line {line_number}", path.display()))?;
            if colors.insert(symbol, color).is_some() {
                bail!(
                    "sprite grid {} line {line_number} defines '{symbol}' more than once",
                    path.display()
                );
            }
        } else {
            if line.is_empty() {
                bail!(
                    "sprite grid {} line {line_number} must not be empty",
                    path.display()
                );
            }
            rows.push((line_number, line));
        }
    }

    if !reading_grid {
        bail!(
            "sprite grid {} is missing the --- separator",
            path.display()
        );
    }
    let Some((_, first_row)) = rows.first() else {
        bail!("sprite grid {} has no pixel rows", path.display());
    };
    let width = first_row.chars().count();
    if width == 0 {
        bail!("sprite grid {} has an empty first row", path.display());
    }
    let width = u32::try_from(width).context("sprite grid width exceeds u32")?;
    let height = u32::try_from(rows.len()).context("sprite grid height exceeds u32")?;
    if width > 4096 || height > 4096 {
        bail!("sprite grid dimensions {width}x{height} exceed the 4096-pixel compiler limit");
    }
    let mut pixels = Vec::with_capacity(width as usize * height as usize);
    for (line_number, row) in rows.iter().copied() {
        let row_width = row.chars().count();
        if row_width != width as usize {
            bail!(
                "sprite grid {} line {line_number} has width {row_width}; expected {width}",
                path.display()
            );
        }
        for symbol in row.chars() {
            if symbol == '.' {
                pixels.push([0, 0, 0, 0]);
            } else if let Some(color) = colors.get(&symbol) {
                pixels.push(*color);
            } else {
                bail!(
                    "sprite grid {} line {line_number} uses undefined symbol '{symbol}'",
                    path.display()
                );
            }
        }
    }
    Ok(SpriteGrid {
        width,
        height,
        pixels,
    })
}

fn parse_palette_entry(line: &str) -> Result<(char, [u8; 4])> {
    let (symbol, color) = line
        .split_once('=')
        .context("palette entry must use SYMBOL=#RRGGBB")?;
    let mut symbols = symbol.chars();
    let Some(symbol) = symbols.next() else {
        bail!("palette symbol must not be empty");
    };
    if symbols.next().is_some() || symbol == '.' || symbol.is_whitespace() {
        bail!("palette symbol must be one non-whitespace character other than '.'");
    }
    let hex = color
        .strip_prefix('#')
        .context("palette color must use #RRGGBB")?;
    if hex.len() != 6 || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        bail!("palette color must use #RRGGBB");
    }
    let red = u8::from_str_radix(&hex[0..2], 16).context("parse red color component")?;
    let green = u8::from_str_radix(&hex[2..4], 16).context("parse green color component")?;
    let blue = u8::from_str_radix(&hex[4..6], 16).context("parse blue color component")?;
    Ok((symbol, [red, green, blue, 0xff]))
}

#[cfg(test)]
#[path = "grid_tests.rs"]
mod tests;
