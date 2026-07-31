use std::collections::{BTreeMap, HashMap};

use serde::Deserialize;
use thiserror::Error;

use crate::color::{FloatColor, Rgba};
use crate::document::{ColorScheme, ColorTransform, ColorValue, ThemeDocument};

const EMBEDDED_MANIFEST: &str = include_str!("../../../resources/design-tokens/design-tokens.json");
const LEGACY_EDITOR_TOKEN_PREFIX: &str = "editor.semanticToken.";
const EDITOR_TOKEN_PREFIX: &str = "editor.token.";

/// Immutable, fully resolved theme selected for one presentation surface.
#[derive(Clone, Debug)]
pub struct ThemeSnapshot {
    id: String,
    label: String,
    color_scheme: ColorScheme,
    colors: BTreeMap<String, Rgba>,
}

impl ThemeSnapshot {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub const fn color_scheme(&self) -> ColorScheme {
        self.color_scheme
    }

    pub fn color(&self, token: &str) -> Option<Rgba> {
        self.colors.get(token).copied()
    }

    pub fn required_color(&self, token: &str) -> Result<Rgba, ThemeError> {
        self.color(token)
            .ok_or_else(|| ThemeError::MissingResolvedColor(token.to_owned()))
    }

    pub fn colors(&self) -> &BTreeMap<String, Rgba> {
        &self.colors
    }
}

/// Versioned catalog compiled from the shared design-token manifest.
pub struct ThemeCatalog {
    colors: BTreeMap<String, ColorContribution>,
}

impl ThemeCatalog {
    pub fn embedded() -> Result<Self, ThemeError> {
        let manifest: Manifest = serde_json::from_str(EMBEDDED_MANIFEST)?;
        if manifest.version != 1 {
            return Err(ThemeError::UnsupportedCatalogVersion(manifest.version));
        }
        let mut colors = BTreeMap::new();
        for contribution in manifest.colors {
            let id = contribution.id.clone();
            if colors.insert(id.clone(), contribution).is_some() {
                return Err(ThemeError::DuplicateToken(id));
            }
        }
        Ok(Self { colors })
    }

    pub fn built_in(&self, color_scheme: ColorScheme) -> Result<ThemeSnapshot, ThemeError> {
        self.resolve(
            color_scheme.built_in_id(),
            color_scheme.built_in_label(),
            color_scheme,
            &BTreeMap::new(),
        )
    }

    pub fn resolve_document(&self, document: &ThemeDocument) -> Result<ThemeSnapshot, ThemeError> {
        let overrides = normalize_legacy_editor_tokens(&document.colors);
        self.resolve(
            document.id(),
            document.label(),
            document.color_scheme(),
            &overrides,
        )
    }

    fn resolve(
        &self,
        id: &str,
        label: &str,
        color_scheme: ColorScheme,
        overrides: &BTreeMap<String, ColorValue>,
    ) -> Result<ThemeSnapshot, ThemeError> {
        for token in overrides.keys() {
            if !self.colors.contains_key(token) {
                return Err(ThemeError::UnknownOverride(token.clone()));
            }
        }
        let mut resolver = Resolver {
            catalog: self,
            color_scheme,
            overrides,
            cache: HashMap::new(),
            resolving: Vec::new(),
        };
        let mut colors = BTreeMap::new();
        for token in self.colors.keys() {
            if let Some(color) = resolver.resolve_token(token)? {
                colors.insert(token.clone(), color.quantized());
            }
        }
        Ok(ThemeSnapshot {
            id: id.to_owned(),
            label: label.to_owned(),
            color_scheme,
            colors,
        })
    }
}

struct Resolver<'a> {
    catalog: &'a ThemeCatalog,
    color_scheme: ColorScheme,
    overrides: &'a BTreeMap<String, ColorValue>,
    cache: HashMap<String, Option<FloatColor>>,
    resolving: Vec<String>,
}

impl Resolver<'_> {
    fn resolve_token(&mut self, id: &str) -> Result<Option<FloatColor>, ThemeError> {
        if let Some(color) = self.cache.get(id) {
            return Ok(*color);
        }
        if let Some(start) = self.resolving.iter().position(|token| token == id) {
            let mut cycle = self.resolving[start..].to_vec();
            cycle.push(id.to_owned());
            return Err(ThemeError::ColorCycle(cycle.join(" -> ")));
        }
        let contribution = self
            .catalog
            .colors
            .get(id)
            .ok_or_else(|| ThemeError::UnknownReference(id.to_owned()))?;
        self.resolving.push(id.to_owned());
        let source = self
            .overrides
            .get(id)
            .or_else(|| contribution.defaults.get(&self.color_scheme))
            .ok_or_else(|| ThemeError::MissingDefault(id.to_owned()))?;
        let color = self.resolve_value(source, 0)?;
        self.resolving.pop();
        if contribution.needs_transparency && color.is_some_and(FloatColor::is_opaque) {
            return Err(ThemeError::TransparencyRequired(id.to_owned()));
        }
        self.cache.insert(id.to_owned(), color);
        Ok(color)
    }

    fn resolve_value(
        &mut self,
        value: &ColorValue,
        depth: usize,
    ) -> Result<Option<FloatColor>, ThemeError> {
        if depth > 8 {
            return Err(ThemeError::TransformDepth);
        }
        match value {
            ColorValue::Missing => Ok(None),
            ColorValue::Reference(reference) if reference.starts_with('#') => {
                FloatColor::parse(reference).map(Some)
            }
            ColorValue::Reference(reference) => self.resolve_token(reference),
            ColorValue::Transform(transform) => {
                self.resolve_transform(transform, depth.saturating_add(1))
            }
        }
    }

    fn resolve_transform(
        &mut self,
        transform: &ColorTransform,
        depth: usize,
    ) -> Result<Option<FloatColor>, ThemeError> {
        let value = match transform {
            ColorTransform::Transparent { value, .. }
            | ColorTransform::Lighten { value, .. }
            | ColorTransform::Darken { value, .. }
            | ColorTransform::Mix { value, .. }
            | ColorTransform::Opaque { value, .. } => self.resolve_value(value, depth)?,
        };
        let Some(value) = value else {
            return Ok(None);
        };
        match transform {
            ColorTransform::Transparent { factor, .. } => {
                valid_factor(*factor)?;
                Ok(Some(value.transparent(*factor)))
            }
            ColorTransform::Lighten { factor, .. } => {
                valid_factor(*factor)?;
                Ok(Some(value.lighten(*factor)))
            }
            ColorTransform::Darken { factor, .. } => {
                valid_factor(*factor)?;
                Ok(Some(value.darken(*factor)))
            }
            ColorTransform::Mix { other, factor, .. } => {
                valid_factor(*factor)?;
                Ok(self
                    .resolve_value(other, depth)?
                    .map(|other| value.mix(other, *factor)))
            }
            ColorTransform::Opaque { background, .. } => Ok(self
                .resolve_value(background, depth)?
                .map(|background| value.make_opaque(background))),
        }
    }
}

fn valid_factor(factor: f64) -> Result<(), ThemeError> {
    if ColorTransform::validate_factor(factor) {
        Ok(())
    } else {
        Err(ThemeError::InvalidFactor)
    }
}

fn normalize_legacy_editor_tokens(
    overrides: &BTreeMap<String, ColorValue>,
) -> BTreeMap<String, ColorValue> {
    let mut normalized = overrides.clone();
    for (id, value) in overrides {
        let Some(suffix) = id.strip_prefix(LEGACY_EDITOR_TOKEN_PREFIX) else {
            continue;
        };
        normalized
            .entry(format!("{EDITOR_TOKEN_PREFIX}{suffix}"))
            .or_insert_with(|| value.clone());
    }
    normalized
}

#[derive(Deserialize)]
struct Manifest {
    version: u8,
    colors: Vec<ColorContribution>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ColorContribution {
    id: String,
    needs_transparency: bool,
    defaults: BTreeMap<ColorScheme, ColorValue>,
}

#[derive(Debug, Error)]
pub enum ThemeError {
    #[error("theme JSON is invalid: {0}")]
    Json(#[from] serde_json::Error),
    #[error("theme document exceeds the 1 MiB limit")]
    DocumentTooLarge,
    #[error("theme document version {0} is unsupported")]
    UnsupportedVersion(u8),
    #[error("design-token catalog version {0} is unsupported")]
    UnsupportedCatalogVersion(u8),
    #[error("theme $schema is invalid")]
    InvalidSchema,
    #[error("theme id '{0}' must be lowercase kebab-case")]
    InvalidThemeId(String),
    #[error("theme label must contain 1 to 80 trimmed characters")]
    InvalidThemeLabel,
    #[error("theme contains more than 512 color overrides")]
    TooManyOverrides,
    #[error("theme color overrides cannot be null")]
    NullOverride,
    #[error("invalid theme color value '{0}'")]
    InvalidColorValue(String),
    #[error("invalid hexadecimal color '{value}'")]
    InvalidColor { value: String },
    #[error("unknown color token override '{0}'")]
    UnknownOverride(String),
    #[error("unknown color token reference '{0}'")]
    UnknownReference(String),
    #[error("color token '{0}' has no default for the selected scheme")]
    MissingDefault(String),
    #[error("resolved theme is missing required color '{0}'")]
    MissingResolvedColor(String),
    #[error("duplicate color token '{0}' in the shared catalog")]
    DuplicateToken(String),
    #[error("color token cycle: {0}")]
    ColorCycle(String),
    #[error("color transform factor must be finite and between 0 and 1")]
    InvalidFactor,
    #[error("color transform exceeds the maximum depth")]
    TransformDepth,
    #[error("color token '{0}' must remain transparent")]
    TransparencyRequired(String),
}
