//! Key-value `.lang` parser, `O(1)` static catalog lookup, and `{placeholder}` formatting.

use std::collections::HashMap;
use std::fmt::Display;

/// Parsed UI translation catalog for a single [`super::UiLanguage`].
pub struct LocaleCatalog {
    messages: HashMap<&'static str, &'static str>,
    ocr_names: HashMap<&'static str, &'static str>,
}

impl LocaleCatalog {
    /// Parses a bundled `.lang` UTF-8 resource into `O(1)` lookup maps.
    ///
    /// Format rules:
    /// - Empty lines and lines starting with `#` are ignored.
    /// - Entries use `key = value` syntax (surrounding whitespace is trimmed).
    /// - Values may optionally be wrapped in `"..."` to preserve leading/trailing spaces,
    ///   and support `\n`, `\t`, `\"`, and `\\` escape sequences.
    pub fn parse(source: &'static str) -> Self {
        let mut messages = HashMap::new();
        let mut ocr_names = HashMap::new();

        for raw_line in source.lines() {
            let line = raw_line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((raw_key, raw_val)) = line.split_once('=') else {
                continue;
            };
            let key = raw_key.trim();
            if key.is_empty() {
                continue;
            }
            let value = parse_value(raw_val.trim());
            messages.insert(key, value);
            if let Some(ocr_code) = key.strip_prefix("ocr.") {
                ocr_names.insert(ocr_code, value);
            }
        }

        Self { messages, ocr_names }
    }

    #[inline]
    pub fn get(&self, key: &str) -> Option<&'static str> {
        self.messages.get(key).copied()
    }

    #[inline]
    pub fn ocr_name(&self, code: &str) -> Option<&'static str> {
        self.ocr_names.get(code).copied()
    }

    pub fn keys(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.messages.keys().copied()
    }
}

fn parse_value(trimmed: &'static str) -> &'static str {
    let unquoted = if trimmed.len() >= 2 && trimmed.starts_with('"') && trimmed.ends_with('"') {
        &trimmed[1..trimmed.len() - 1]
    } else {
        trimmed
    };

    if !unquoted.contains('\\') {
        return unquoted;
    }

    let mut out = String::with_capacity(unquoted.len());
    let mut chars = unquoted.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.next() {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some('"') => out.push('"'),
                Some('\\') => out.push('\\'),
                Some(other) => {
                    out.push('\\');
                    out.push(other);
                }
                None => out.push('\\'),
            }
        } else {
            out.push(ch);
        }
    }

    Box::leak(out.into_boxed_str())
}

/// Replaces `{name}` placeholders in `template` using `args`.
/// Unknown placeholders are preserved verbatim so partial templates never panic.
pub fn interpolate(template: &str, args: &[(&str, &dyn Display)]) -> String {
    if args.is_empty() || !template.contains('{') {
        return template.to_owned();
    }

    let mut out = String::with_capacity(template.len() + 16);
    let mut rest = template;

    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        let after_open = &rest[open + 1..];
        if let Some(close) = after_open.find('}') {
            let token = &after_open[..close];
            if let Some((_, val)) = args.iter().find(|&&(k, _)| k == token) {
                use std::fmt::Write as _;
                let _ = write!(out, "{val}");
            } else {
                out.push('{');
                out.push_str(token);
                out.push('}');
            }
            rest = &after_open[close + 1..];
        } else {
            out.push_str(&rest[open..]);
            rest = "";
        }
    }

    out.push_str(rest);
    out
}

#[cfg(test)]
pub(crate) fn extract_placeholders(text: &str) -> Vec<&str> {
    let mut names = Vec::new();
    let mut rest = text;
    while let Some(open) = rest.find('{') {
        let after = &rest[open + 1..];
        let Some(close) = after.find('}') else {
            break;
        };
        let token = &after[..close];
        if !token.is_empty() && token.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') {
            names.push(token);
        }
        rest = &after[close + 1..];
    }
    names.sort_unstable();
    names.dedup();
    names
}
