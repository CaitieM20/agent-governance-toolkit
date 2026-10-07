// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

//! Parser-independent policy values. YAML policies use the JSON data model:
//! string mapping keys, finite numbers, booleans, null, arrays and objects.

pub use serde_json::Value;
/// A nested policy mapping.
pub type Mapping = serde_json::Map<String, Value>;
/// Context supplied to policy evaluation and protocol extractors.
pub type Context = std::collections::HashMap<String, Value>;

/// A YAML configuration error, including a source location when available.
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct YamlError(#[source] YamlErrorSource);

#[derive(Debug, thiserror::Error)]
enum YamlErrorSource {
    #[error("{0}")]
    Deserialize(#[source] Box<serde_saphyr::Error>),
    #[error("{0}")]
    Normalize(#[source] YamlScalarError),
}

#[derive(Debug)]
enum YamlScalarError {
    Scan(serde_saphyr::granit_parser::ScanError),
    Invalid(String),
    ResourceLimit(String),
}

impl YamlScalarError {
    fn location(&self) -> Option<(u64, u64)> {
        match self {
            Self::Scan(error) => Some((
                error.marker().line() as u64,
                error.marker().col() as u64 + 1,
            )),
            Self::Invalid(_) | Self::ResourceLimit(_) => None,
        }
    }
}

impl std::fmt::Display for YamlScalarError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Scan(error) => error.fmt(formatter),
            Self::Invalid(detail) | Self::ResourceLimit(detail) => formatter.write_str(detail),
        }
    }
}

impl std::error::Error for YamlScalarError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Scan(error) => Some(error),
            Self::Invalid(_) | Self::ResourceLimit(_) => None,
        }
    }
}

impl YamlError {
    /// One-based line and column, if the parser supplied a location.
    pub fn location(&self) -> Option<(u64, u64)> {
        match &self.0 {
            YamlErrorSource::Deserialize(error) => {
                error.location().map(|loc| (loc.line(), loc.column()))
            }
            YamlErrorSource::Normalize(error) => error.location(),
        }
    }
}

impl From<serde_saphyr::Error> for YamlError {
    fn from(error: serde_saphyr::Error) -> Self {
        Self(YamlErrorSource::Deserialize(Box::new(error)))
    }
}

pub(crate) const MAX_YAML_BYTES: usize = 1_048_576;

fn normalize_yaml_scalars(
    input: &str,
    max_depth: usize,
    max_events: usize,
) -> Result<std::borrow::Cow<'_, str>, YamlScalarError> {
    use serde_saphyr::granit_parser::{self, Event, Parser, ScalarStyle};

    let parser = Parser::new_from_str_with_options(
        input,
        granit_parser::options! {
            emit_comments: false,
            flow_nesting_limit: max_depth,
            block_nesting_limit: max_depth,
        },
    );
    let mut output = String::new();
    let mut copied = 0;
    for (events, event) in parser.enumerate() {
        if events >= max_events {
            return Err(YamlScalarError::ResourceLimit(
                "YAML parser event limit exceeded".to_string(),
            ));
        }
        let (event, span) = event.map_err(YamlScalarError::Scan)?;
        let tag = match &event {
            Event::Scalar(_, _, _, tag)
            | Event::MappingStart(_, _, tag)
            | Event::SequenceStart(_, _, tag) => tag.as_ref(),
            _ => None,
        };
        if tag.is_some_and(|tag| tag.handle().is_empty() && tag.suffix() == "!") {
            return Err(YamlScalarError::Invalid(format!(
                "non-specific YAML tags are not supported at line {}, column {}",
                span.start.line(),
                span.start.col() + 1
            )));
        }
        let Event::Scalar(value, style, _, tag) = event else {
            continue;
        };
        let boolean_tag = tag
            .as_ref()
            .is_some_and(|tag| tag.is_yaml_core_schema_tag("bool"));
        let kind = tag.as_ref().and_then(|tag| tag.core_suffix());
        let string_tag = kind == Some("str");
        let integer_tag = kind == Some("int");
        let float_tag = kind == Some("float");
        let null_tag = kind == Some("null");
        let invalid = |message: &str| {
            YamlScalarError::Invalid(format!(
                "{message} at line {}, column {}",
                span.start.line(),
                span.start.col() + 1
            ))
        };
        if tag.is_some() && !boolean_tag && !string_tag && !integer_tag && !float_tag && !null_tag {
            return Err(invalid("unsupported YAML scalar tag"));
        }
        if style != ScalarStyle::Plain && (tag.is_none() || string_tag) {
            continue;
        }
        let boolean = if tag
            .as_ref()
            .is_none_or(|tag| tag.is_yaml_core_schema_tag("bool"))
        {
            match value.as_ref() {
                "true" | "True" | "TRUE" => Some("true"),
                "false" | "False" | "FALSE" => Some("false"),
                _ => None,
            }
        } else {
            None
        };
        if boolean_tag && boolean.is_none() {
            return Err(invalid("invalid YAML boolean"));
        }
        let mut explicit_scalar = None;
        if null_tag {
            if !matches!(value.as_ref(), "" | "~" | "null" | "Null" | "NULL") {
                return Err(invalid("invalid YAML null"));
            }
            explicit_scalar = Some("null".to_string());
        }
        if float_tag {
            let number = value
                .parse::<f64>()
                .map_err(|_| invalid("invalid YAML float"))?;
            if !number.is_finite() {
                return Err(invalid("YAML numbers must be finite"));
            }
            explicit_scalar =
                Some(serde_json::to_string(&number).map_err(|_| invalid("invalid YAML float"))?);
        }
        let mixed_keyword = (boolean.is_none()
            && (value.eq_ignore_ascii_case("true") || value.eq_ignore_ascii_case("false")))
            || (value.eq_ignore_ascii_case("null")
                && !matches!(value.as_ref(), "null" | "Null" | "NULL"));
        let unsigned = value.strip_prefix(['+', '-']).unwrap_or(&value);
        let leading_zero = unsigned.len() > 1
            && unsigned.starts_with('0')
            && unsigned.bytes().all(|byte| byte.is_ascii_digit());
        let (digits, radix) = if let Some(digits) = unsigned.strip_prefix("0x") {
            (digits, 16)
        } else if let Some(digits) = unsigned.strip_prefix("0o") {
            (digits, 8)
        } else if let Some(digits) = unsigned.strip_prefix("0b") {
            (digits, 2)
        } else {
            (unsigned, 10)
        };
        if !string_tag
            && !float_tag
            && !leading_zero
            && !digits.is_empty()
            && digits.chars().all(|ch| ch.is_digit(radix))
        {
            let magnitude = u64::from_str_radix(digits, radix).map_err(|_| {
                YamlScalarError::Invalid(
                    "YAML integer exceeds the supported 64-bit range".to_string(),
                )
            })?;
            if value.starts_with('-') && magnitude > (i64::MAX as u64) + 1 {
                return Err(YamlScalarError::Invalid(
                    "YAML integer exceeds the supported 64-bit range".to_string(),
                ));
            }
            if integer_tag {
                explicit_scalar = Some(if value.starts_with('-') {
                    format!("-{magnitude}")
                } else {
                    magnitude.to_string()
                });
            }
        }
        if integer_tag && explicit_scalar.is_none() {
            return Err(invalid("invalid YAML integer"));
        }
        let separated_number = unsigned.contains('_')
            && unsigned.starts_with(|ch: char| ch.is_ascii_digit() || ch == '.')
            && !unsigned.chars().any(char::is_whitespace);
        let legacy_prefix = [("0X", 16), ("0O", 8), ("0B", 2)]
            .iter()
            .any(|(prefix, radix)| {
                unsigned.strip_prefix(*prefix).is_some_and(|digits| {
                    !digits.is_empty() && digits.chars().all(|ch| ch.is_digit(*radix))
                })
            });
        if !leading_zero
            && !separated_number
            && !legacy_prefix
            && boolean.is_none()
            && !mixed_keyword
            && !string_tag
            && explicit_scalar.is_none()
        {
            continue;
        }
        let range = span.byte_range().ok_or_else(|| {
            YamlScalarError::Invalid("missing YAML scalar source range".to_string())
        })?;
        let block = matches!(style, ScalarStyle::Literal | ScalarStyle::Folded);
        let mut replacement_start = range.start;
        let mut prefix_start = copied;
        if tag.is_some() && !string_tag {
            let start = span
                .tag_start()
                .and_then(|marker| marker.byte_offset())
                .ok_or_else(|| {
                    YamlScalarError::Invalid("missing YAML tag source range".to_string())
                })?;
            let prefix = input.get(copied..start).ok_or_else(|| {
                YamlScalarError::Invalid("invalid YAML tag source range".to_string())
            })?;
            let tag_source = input.get(start..range.start).ok_or_else(|| {
                YamlScalarError::Invalid("invalid YAML tag source range".to_string())
            })?;
            let length = tag_source
                .find(char::is_whitespace)
                .unwrap_or(tag_source.len());
            output.push_str(prefix);
            output.extend(std::iter::repeat_n(' ', length));
            prefix_start = start + length;
        }
        if block {
            let mut cursor = prefix_start;
            loop {
                let remaining = input
                    .get(cursor..range.end)
                    .ok_or_else(|| invalid("invalid YAML block scalar range"))?;
                let ch = remaining
                    .chars()
                    .next()
                    .ok_or_else(|| invalid("missing YAML block scalar header"))?;
                match ch {
                    '|' | '>' => {
                        replacement_start = cursor;
                        break;
                    }
                    '#' => cursor += remaining.find('\n').unwrap_or(remaining.len()),
                    '&' => {
                        cursor += remaining
                            .find(char::is_whitespace)
                            .ok_or_else(|| invalid("invalid YAML block scalar properties"))?
                    }
                    ch if ch.is_whitespace() => cursor += ch.len_utf8(),
                    _ => return Err(invalid("invalid YAML block scalar header")),
                }
            }
        }
        let prefix = input.get(prefix_start..replacement_start).ok_or_else(|| {
            YamlScalarError::Invalid("invalid YAML scalar source range".to_string())
        })?;
        output.push_str(prefix);
        if string_tag && range.is_empty() {
            output.push(' ');
        }
        if let Some(explicit_scalar) = explicit_scalar {
            output.push_str(&explicit_scalar);
        } else if let Some(boolean) = boolean {
            output.push_str(boolean);
        } else {
            output.push_str(
                &serde_json::to_string(if string_tag && range.is_empty() {
                    ""
                } else {
                    value.as_ref()
                })
                .map_err(|error| YamlScalarError::Invalid(error.to_string()))?,
            );
        }
        if block {
            let replaced = input
                .get(replacement_start..range.end)
                .ok_or_else(|| invalid("invalid YAML block scalar range"))?;
            let tail = &replaced[replaced.trim_end_matches(char::is_whitespace).len()..];
            let removed_lines = replaced.bytes().filter(|byte| *byte == b'\n').count();
            let tail_lines = tail.bytes().filter(|byte| *byte == b'\n').count();
            output.extend(std::iter::repeat_n(
                '\n',
                removed_lines.saturating_sub(tail_lines),
            ));
            output.push_str(tail);
        }
        copied = range.end;
    }
    if copied == 0 {
        Ok(std::borrow::Cow::Borrowed(input))
    } else {
        output.push_str(&input[copied..]);
        Ok(std::borrow::Cow::Owned(output))
    }
}

pub(crate) fn from_yaml<T: serde::de::DeserializeOwned>(input: &str) -> Result<T, YamlError> {
    if input.len() > MAX_YAML_BYTES {
        return Err(serde::de::Error::custom(
            "YAML configuration exceeds 1048576 bytes",
        ));
    }
    let normalized = normalize_yaml_scalars(input, 64, 300_000)
        .map_err(|error| YamlError(YamlErrorSource::Normalize(error)))?;
    let value: serde_json::Value = serde_saphyr::from_str_with_options(
        &normalized,
        serde_saphyr::options! {
            emit_comments: false,
            strict_booleans: true,
            no_schema: true,
            reject_unsupported_tags: true,
            merge_keys: serde_saphyr::MergeKeyPolicy::AsOrdinary,
            with_snippet: false,
            budget: serde_saphyr::budget! {
                max_depth: 64,
                max_nodes: 100_000,
                max_events: 300_000,
                max_total_scalar_bytes: MAX_YAML_BYTES,
                max_recorded_anchor_bytes: MAX_YAML_BYTES,
                max_recorded_anchor_events: 100_000,
            },
        },
    )
    .map_err(YamlError::from)?;
    T::deserialize(ConfigValue(value)).map_err(<YamlError as serde::de::Error>::custom)
}

// JSON's default struct decoder accepts positional arrays. Configuration structs
// must be mappings, including structs inside sequences, options and enum payloads.
struct ConfigValue(Value);

impl<'de> serde::de::IntoDeserializer<'de, serde_json::Error> for ConfigValue {
    type Deserializer = Self;

    fn into_deserializer(self) -> Self {
        self
    }
}

impl<'de> serde::Deserializer<'de> for ConfigValue {
    type Error = serde_json::Error;

    fn deserialize_any<V: serde::de::Visitor<'de>>(
        self,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        use serde::de::value::{MapDeserializer, SeqDeserializer};
        match self.0 {
            Value::Array(values) => {
                SeqDeserializer::new(values.into_iter().map(ConfigValue)).deserialize_any(visitor)
            }
            Value::Object(values) => MapDeserializer::new(
                values
                    .into_iter()
                    .map(|(key, value)| (key, ConfigValue(value))),
            )
            .deserialize_any(visitor),
            value => value.deserialize_any(visitor),
        }
    }

    fn deserialize_map<V: serde::de::Visitor<'de>>(
        self,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        if self.0.is_object() {
            self.deserialize_any(visitor)
        } else {
            self.0.deserialize_map(visitor)
        }
    }

    fn deserialize_struct<V: serde::de::Visitor<'de>>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.deserialize_map(visitor)
    }

    fn deserialize_option<V: serde::de::Visitor<'de>>(
        self,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        if self.0.is_null() {
            visitor.visit_none()
        } else {
            visitor.visit_some(self)
        }
    }

    fn deserialize_newtype_struct<V: serde::de::Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_enum<V: serde::de::Visitor<'de>>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        use serde::de::value::{MapAccessDeserializer, MapDeserializer};
        match self.0 {
            Value::Object(values) if values.len() == 1 => {
                MapAccessDeserializer::new(MapDeserializer::new(
                    values
                        .into_iter()
                        .map(|(key, value)| (key, ConfigValue(value))),
                ))
                .deserialize_enum(name, variants, visitor)
            }
            value => value.deserialize_enum(name, variants, visitor),
        }
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf unit unit_struct seq tuple tuple_struct identifier ignored_any
    }
}

impl serde::de::Error for YamlError {
    fn custom<T: std::fmt::Display>(message: T) -> Self {
        Self::from(<serde_saphyr::Error as serde::de::Error>::custom(message))
    }
}

pub(crate) fn read_yaml(path: impl AsRef<std::path::Path>) -> std::io::Result<String> {
    use std::io::Read;
    let mut input = String::new();
    std::fs::File::open(path)?
        .take(MAX_YAML_BYTES as u64 + 1)
        .read_to_string(&mut input)?;
    if input.len() > MAX_YAML_BYTES {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "YAML configuration exceeds 1048576 bytes",
        ));
    }
    Ok(input)
}
