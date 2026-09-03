use anyhow::Context;
use anyhow::Result;
use anyhow::bail;
use clap::Parser;
use std::path::Path;
use std::path::PathBuf;
use zeta_sprite::compiler::ansi_preview;
use zeta_sprite::compiler::rasterize;
use zeta_sprite::compiler::rust_source;
use zeta_sprite::compiler::source_dimensions;
use zeta_sprite::pack_quadrants_rgba;

#[derive(Debug, Parser)]
#[command(about = "Convert an SVG, PNG, or pixel grid into a terminal Unicode sprite")]
struct Args {
    /// SVG, PNG, or .sprite pixel-grid design source.
    input: PathBuf,

    /// Output width in terminal cells.
    #[arg(long, value_parser = positive_u32)]
    columns: Option<u32>,

    /// Output height in terminal cells.
    #[arg(long, value_parser = positive_u32)]
    rows: Option<u32>,

    /// Write a checked-in Rust sprite constant to this path.
    #[arg(long = "rust")]
    rust_output: Option<PathBuf>,

    /// Verify the checked-in Rust sprite instead of writing it.
    #[arg(long, requires = "rust_output")]
    check: bool,

    /// Rust constant name used with --rust.
    #[arg(long, default_value = "PET", value_parser = constant_name)]
    name: String,

    /// Alpha values below this threshold become transparent.
    #[arg(long, default_value_t = 128, value_parser = clap::value_parser!(u8).range(1..))]
    alpha_threshold: u8,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let (source_width, source_height) = source_dimensions(&args.input)?;
    let (columns, rows) =
        terminal_dimensions(source_width, source_height, args.columns, args.rows)?;
    let logical_width = columns
        .checked_mul(2)
        .context("terminal sprite width is too large")?;
    let logical_height = rows
        .checked_mul(2)
        .context("terminal sprite height is too large")?;
    let image = rasterize(&args.input, logical_width, logical_height)?;
    let sprite = pack_quadrants_rgba(
        image.width,
        image.height,
        &image.pixels,
        args.alpha_threshold,
    )?;

    if !args.check {
        print!("{}", ansi_preview(sprite.as_sprite()));
        eprintln!(
            "{}x{} source -> {}x{} logical pixels -> {}x{} terminal cells",
            source_width, source_height, logical_width, logical_height, columns, rows
        );
    }

    if let Some(path) = args.rust_output {
        let source = rust_source(&args.name, &sprite);
        if args.check {
            check_rust_output(&path, &source)?;
            eprintln!("checked {}", path.display());
        } else {
            if let Some(parent) = path
                .parent()
                .filter(|parent| !parent.as_os_str().is_empty())
            {
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("create output directory {}", parent.display()))?;
            }
            std::fs::write(&path, source)
                .with_context(|| format!("write Rust sprite {}", path.display()))?;
            eprintln!("wrote {}", path.display());
        }
    }
    Ok(())
}

fn check_rust_output(path: &Path, expected: &str) -> Result<()> {
    let actual = std::fs::read_to_string(path)
        .with_context(|| format!("read Rust sprite {}", path.display()))?;
    if actual != expected {
        bail!(
            "Rust sprite {} is out of date; regenerate it without --check",
            path.display()
        );
    }
    Ok(())
}

fn terminal_dimensions(
    source_width: u32,
    source_height: u32,
    columns: Option<u32>,
    rows: Option<u32>,
) -> Result<(u32, u32)> {
    let dimensions = match (columns, rows) {
        (Some(columns), Some(rows)) => (columns, rows),
        (Some(columns), None) => (
            columns,
            ceil_ratio(
                u64::from(columns) * u64::from(source_height),
                u64::from(source_width) * 2,
            ),
        ),
        (None, Some(rows)) => (
            ceil_ratio(
                u64::from(rows) * u64::from(source_width) * 2,
                u64::from(source_height),
            ),
            rows,
        ),
        (None, None) => (source_width, source_height.div_ceil(2)),
    };
    if dimensions.0 == 0 || dimensions.1 == 0 {
        bail!("terminal sprite dimensions must be non-zero");
    }
    if dimensions.0 > u32::from(u16::MAX) || dimensions.1 > u32::from(u16::MAX) {
        bail!("terminal sprite dimensions exceed u16");
    }
    Ok(dimensions)
}

fn ceil_ratio(numerator: u64, denominator: u64) -> u32 {
    u32::try_from(numerator.div_ceil(denominator).max(1)).unwrap_or(u32::MAX)
}

fn positive_u32(value: &str) -> Result<u32, String> {
    value
        .parse::<u32>()
        .map_err(|error| error.to_string())
        .and_then(|value| {
            if value == 0 {
                Err("value must be greater than zero".into())
            } else {
                Ok(value)
            }
        })
}

fn constant_name(value: &str) -> Result<String, String> {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return Err("constant name must not be empty".into());
    };
    if !(first.is_ascii_uppercase() || first == '_')
        || !chars.all(|character| {
            character.is_ascii_uppercase() || character.is_ascii_digit() || character == '_'
        })
    {
        return Err("constant name must be an uppercase Rust identifier".into());
    }
    Ok(value.into())
}

#[cfg(test)]
#[path = "main_tests.rs"]
mod tests;
