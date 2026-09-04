//! Parsing and compilation for editable named sprite frames and actions.

use crate::OwnedTerminalSprite;
use crate::pack_octants_rgba;
use anyhow::Context;
use anyhow::Result;
use anyhow::bail;
use std::collections::BTreeMap;
use std::path::Path;

const FORMAT_VERSION: &str = "version 1";
const MAX_DIMENSION: u32 = 4096;
const MAX_ACTION_STEPS: usize = 8;
const MAX_ACTION_DURATION_MS: u16 = 600;
const MIN_STEP_DURATION_MS: u16 = 25;
const MAX_STEP_DURATION_MS: u16 = 250;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SpriteFrameSource {
    pub(crate) name: String,
    pub(crate) pixels: Vec<[u8; 4]>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SpriteSheetSource {
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) frames: Vec<SpriteFrameSource>,
    pub(crate) actions: Vec<SpriteActionSource>,
    pub(crate) idle_frame_index: u16,
}

impl SpriteSheetSource {
    #[cfg(feature = "compiler")]
    pub(crate) fn idle(&self) -> &SpriteFrameSource {
        &self.frames[usize::from(self.idle_frame_index)]
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SpriteActionSource {
    pub(crate) name: String,
    pub(crate) steps: Vec<CompiledSpriteActionStep>,
}

/// One named, compiled terminal frame.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompiledSpriteFrame {
    name: String,
    sprite: OwnedTerminalSprite,
}

impl CompiledSpriteFrame {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn sprite(&self) -> &OwnedTerminalSprite {
        &self.sprite
    }
}

/// One frame reference and duration in a compiled sprite action.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CompiledSpriteActionStep {
    frame_index: u16,
    duration_ms: u16,
}

impl CompiledSpriteActionStep {
    pub fn frame_index(self) -> u16 {
        self.frame_index
    }

    pub fn duration_ms(self) -> u16 {
        self.duration_ms
    }
}

/// One named sequence of compiled sprite frames.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompiledSpriteAction {
    name: String,
    steps: Vec<CompiledSpriteActionStep>,
}

impl CompiledSpriteAction {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn steps(&self) -> &[CompiledSpriteActionStep] {
        &self.steps
    }
}

/// Fully validated frames and action timings from one `.sprite` source.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompiledSpriteSheet {
    source_width: u32,
    source_height: u32,
    frames: Vec<CompiledSpriteFrame>,
    actions: Vec<CompiledSpriteAction>,
    idle_frame_index: u16,
}

impl CompiledSpriteSheet {
    pub fn source_dimensions(&self) -> (u32, u32) {
        (self.source_width, self.source_height)
    }

    pub fn frames(&self) -> &[CompiledSpriteFrame] {
        &self.frames
    }

    pub fn actions(&self) -> &[CompiledSpriteAction] {
        &self.actions
    }

    pub fn idle_frame_index(&self) -> u16 {
        self.idle_frame_index
    }

    pub fn idle(&self) -> &CompiledSpriteFrame {
        &self.frames[usize::from(self.idle_frame_index)]
    }

    pub fn action(&self, name: &str) -> Option<&CompiledSpriteAction> {
        self.actions.iter().find(|action| action.name == name)
    }
}

/// Parses and packs every named frame and action in a `.sprite` design source.
pub fn compile_sprite_sheet(path: &Path, alpha_threshold: u8) -> Result<CompiledSpriteSheet> {
    let source = read_sprite_sheet(path)?;
    let frames = source
        .frames
        .into_iter()
        .map(|frame| {
            let sprite =
                pack_octants_rgba(source.width, source.height, &frame.pixels, alpha_threshold)
                    .with_context(|| format!("compile frame '{}'", frame.name))?;
            Ok(CompiledSpriteFrame {
                name: frame.name,
                sprite,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let actions = source
        .actions
        .into_iter()
        .map(|action| CompiledSpriteAction {
            name: action.name,
            steps: action.steps,
        })
        .collect();
    Ok(CompiledSpriteSheet {
        source_width: source.width,
        source_height: source.height,
        frames,
        actions,
        idle_frame_index: source.idle_frame_index,
    })
}

pub(crate) fn read_sprite_sheet(path: &Path) -> Result<SpriteSheetSource> {
    let contents = std::fs::read_to_string(path)
        .with_context(|| format!("read sprite sheet {}", path.display()))?;
    parse_sprite_sheet(path, &contents)
}

fn parse_sprite_sheet(path: &Path, contents: &str) -> Result<SpriteSheetSource> {
    let mut lines = contents.lines().enumerate().peekable();
    let Some((_, version)) = lines.next() else {
        bail!("sprite sheet {} is empty", path.display());
    };
    if version != FORMAT_VERSION {
        bail!(
            "sprite sheet {} line 1 must be '{FORMAT_VERSION}'",
            path.display()
        );
    }
    let (size_line, size) = next_content_line(&mut lines)
        .with_context(|| format!("sprite sheet {} is missing its size", path.display()))?;
    let (width, height) =
        parse_size(size).with_context(|| format!("parse {} line {size_line}", path.display()))?;

    let mut colors = BTreeMap::new();
    let mut frames = Vec::new();
    let mut frame_indices = BTreeMap::new();
    let mut unresolved_actions = Vec::new();
    let mut action_names = BTreeMap::new();
    let mut actions_started = false;

    while let Some((index, line)) = lines.next() {
        let line_number = index + 1;
        if line.is_empty() {
            continue;
        }
        if let Some(entry) = line.strip_prefix("color ") {
            if !frames.is_empty() || actions_started {
                bail!(
                    "sprite sheet {} line {line_number} defines a color after its frames",
                    path.display()
                );
            }
            let (symbol, color) = parse_color(entry)
                .with_context(|| format!("parse {} line {line_number}", path.display()))?;
            if colors.insert(symbol, color).is_some() {
                bail!(
                    "sprite sheet {} line {line_number} defines '{symbol}' more than once",
                    path.display()
                );
            }
            continue;
        }
        if let Some(name) = line.strip_prefix("frame ") {
            if actions_started {
                bail!(
                    "sprite sheet {} line {line_number} defines a frame after its actions",
                    path.display()
                );
            }
            validate_name(name)
                .with_context(|| format!("parse {} line {line_number}", path.display()))?;
            if frame_indices.contains_key(name) {
                bail!(
                    "sprite sheet {} line {line_number} defines frame '{name}' more than once",
                    path.display()
                );
            }
            let pixels = parse_frame(path, &mut lines, name, width, height, &colors)?;
            let frame_index =
                u16::try_from(frames.len()).context("sprite frame count exceeds u16")?;
            frame_indices.insert(name.to_owned(), frame_index);
            frames.push(SpriteFrameSource {
                name: name.to_owned(),
                pixels,
            });
            continue;
        }
        if let Some(name) = line.strip_prefix("action ") {
            actions_started = true;
            validate_name(name)
                .with_context(|| format!("parse {} line {line_number}", path.display()))?;
            if action_names.insert(name.to_owned(), line_number).is_some() {
                bail!(
                    "sprite sheet {} line {line_number} defines action '{name}' more than once",
                    path.display()
                );
            }
            unresolved_actions.push(parse_action(path, &mut lines, name)?);
            continue;
        }
        bail!(
            "sprite sheet {} line {line_number} must define a color, frame, or action",
            path.display()
        );
    }

    if colors.is_empty() {
        bail!("sprite sheet {} defines no colors", path.display());
    }
    if frames.is_empty() {
        bail!("sprite sheet {} defines no frames", path.display());
    }
    let Some(&idle_frame_index) = frame_indices.get("idle") else {
        bail!("sprite sheet {} must define frame 'idle'", path.display());
    };
    let actions = resolve_actions(path, unresolved_actions, &frame_indices)?;
    Ok(SpriteSheetSource {
        width,
        height,
        frames,
        actions,
        idle_frame_index,
    })
}

fn next_content_line<'a, I>(lines: &mut std::iter::Peekable<I>) -> Option<(usize, &'a str)>
where
    I: Iterator<Item = (usize, &'a str)>,
{
    lines
        .find(|(_, line)| !line.is_empty())
        .map(|(index, line)| (index + 1, line))
}

fn parse_size(line: &str) -> Result<(u32, u32)> {
    let values = line.split_whitespace().collect::<Vec<_>>();
    if values.len() != 3 || values[0] != "size" {
        bail!("size must use 'size WIDTH HEIGHT'");
    }
    let width = values[1].parse::<u32>().context("parse sprite width")?;
    let height = values[2].parse::<u32>().context("parse sprite height")?;
    if width == 0 || height == 0 {
        bail!("sprite dimensions must be non-zero");
    }
    if width > MAX_DIMENSION || height > MAX_DIMENSION {
        bail!("sprite dimensions {width}x{height} exceed the {MAX_DIMENSION}-pixel compiler limit");
    }
    Ok((width, height))
}

fn parse_color(entry: &str) -> Result<(char, [u8; 4])> {
    let values = entry.split_whitespace().collect::<Vec<_>>();
    if values.len() != 2 {
        bail!("color must use 'color SYMBOL #RRGGBB'");
    }
    let mut symbols = values[0].chars();
    let Some(symbol) = symbols.next() else {
        bail!("color symbol must not be empty");
    };
    if symbols.next().is_some() || symbol == '.' || symbol.is_whitespace() || !symbol.is_ascii() {
        bail!("color symbol must be one non-whitespace ASCII character other than '.'");
    }
    let hex = values[1]
        .strip_prefix('#')
        .context("color must use #RRGGBB")?;
    if hex.len() != 6 || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        bail!("color must use #RRGGBB");
    }
    Ok((
        symbol,
        [
            u8::from_str_radix(&hex[0..2], 16).context("parse red color component")?,
            u8::from_str_radix(&hex[2..4], 16).context("parse green color component")?,
            u8::from_str_radix(&hex[4..6], 16).context("parse blue color component")?,
            0xff,
        ],
    ))
}

fn parse_frame<'a, I>(
    path: &Path,
    lines: &mut std::iter::Peekable<I>,
    name: &str,
    width: u32,
    height: u32,
    colors: &BTreeMap<char, [u8; 4]>,
) -> Result<Vec<[u8; 4]>>
where
    I: Iterator<Item = (usize, &'a str)>,
{
    let mut pixels = Vec::with_capacity(width as usize * height as usize);
    let mut row_count = 0usize;
    for (index, row) in lines.by_ref() {
        let line_number = index + 1;
        if row == "end" {
            if row_count != height as usize {
                bail!(
                    "sprite sheet {} frame '{name}' has {row_count} rows; expected {height}",
                    path.display()
                );
            }
            return Ok(pixels);
        }
        if row.is_empty() {
            bail!(
                "sprite sheet {} line {line_number} must not be empty inside frame '{name}'",
                path.display()
            );
        }
        if row_count >= height as usize {
            bail!(
                "sprite sheet {} frame '{name}' has more than {height} rows",
                path.display()
            );
        }
        let row_width = row.chars().count();
        if row_width != width as usize {
            bail!(
                "sprite sheet {} line {line_number} in frame '{name}' has width {row_width}; expected {width}",
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
                    "sprite sheet {} line {line_number} in frame '{name}' uses undefined symbol '{symbol}'",
                    path.display()
                );
            }
        }
        row_count += 1;
    }
    bail!(
        "sprite sheet {} frame '{name}' is missing its end",
        path.display()
    )
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct UnresolvedAction {
    name: String,
    steps: Vec<(String, u16, usize)>,
}

fn parse_action<'a, I>(
    path: &Path,
    lines: &mut std::iter::Peekable<I>,
    name: &str,
) -> Result<UnresolvedAction>
where
    I: Iterator<Item = (usize, &'a str)>,
{
    let mut steps = Vec::new();
    let mut total_duration = 0u16;
    for (index, line) in lines.by_ref() {
        let line_number = index + 1;
        if line == "end" {
            if steps.is_empty() {
                bail!(
                    "sprite sheet {} action '{name}' has no steps",
                    path.display()
                );
            }
            return Ok(UnresolvedAction {
                name: name.to_owned(),
                steps,
            });
        }
        if line.is_empty() {
            bail!(
                "sprite sheet {} line {line_number} must not be empty inside action '{name}'",
                path.display()
            );
        }
        if steps.len() == MAX_ACTION_STEPS {
            bail!(
                "sprite sheet {} action '{name}' exceeds {MAX_ACTION_STEPS} steps",
                path.display()
            );
        }
        let values = line.split_whitespace().collect::<Vec<_>>();
        if values.len() != 2 {
            bail!(
                "sprite sheet {} line {line_number} action step must use 'FRAME DURATION_MS'",
                path.display()
            );
        }
        validate_name(values[0])
            .with_context(|| format!("parse {} line {line_number}", path.display()))?;
        let duration_ms = values[1]
            .parse::<u16>()
            .with_context(|| format!("parse {} line {line_number} duration", path.display()))?;
        if !(MIN_STEP_DURATION_MS..=MAX_STEP_DURATION_MS).contains(&duration_ms)
            || duration_ms % MIN_STEP_DURATION_MS != 0
        {
            bail!(
                "sprite sheet {} line {line_number} duration must be a multiple of {MIN_STEP_DURATION_MS}ms between {MIN_STEP_DURATION_MS}ms and {MAX_STEP_DURATION_MS}ms",
                path.display()
            );
        }
        total_duration = total_duration
            .checked_add(duration_ms)
            .context("sprite action duration exceeds u16")?;
        if total_duration > MAX_ACTION_DURATION_MS {
            bail!(
                "sprite sheet {} action '{name}' exceeds {MAX_ACTION_DURATION_MS}ms",
                path.display()
            );
        }
        steps.push((values[0].to_owned(), duration_ms, line_number));
    }
    bail!(
        "sprite sheet {} action '{name}' is missing its end",
        path.display()
    )
}

fn resolve_actions(
    path: &Path,
    actions: Vec<UnresolvedAction>,
    frame_indices: &BTreeMap<String, u16>,
) -> Result<Vec<SpriteActionSource>> {
    actions
        .into_iter()
        .map(|action| {
            let steps = action
                .steps
                .into_iter()
                .map(|(frame, duration_ms, line_number)| {
                    if frame == "idle" {
                        bail!(
                            "sprite sheet {} line {line_number} action '{}' must not reference idle; actions return to idle automatically",
                            path.display(),
                            action.name
                        );
                    }
                    let Some(&frame_index) = frame_indices.get(&frame) else {
                        bail!(
                            "sprite sheet {} line {line_number} action '{}' references unknown frame '{frame}'",
                            path.display(),
                            action.name
                        );
                    };
                    Ok(CompiledSpriteActionStep {
                        frame_index,
                        duration_ms,
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            Ok(SpriteActionSource {
                name: action.name,
                steps,
            })
        })
        .collect()
}

fn validate_name(name: &str) -> Result<()> {
    let mut segments = name.split('-');
    if name.is_empty()
        || segments.any(|segment| {
            segment.is_empty()
                || !segment.starts_with(|character: char| character.is_ascii_lowercase())
                || !segment
                    .chars()
                    .all(|character| character.is_ascii_lowercase() || character.is_ascii_digit())
        })
    {
        bail!("name '{name}' must use lowercase kebab-case");
    }
    Ok(())
}

#[cfg(test)]
#[path = "sheet_tests.rs"]
mod tests;
