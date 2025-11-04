use rand::{seq::SliceRandom, thread_rng};

pub const DEFAULT_FILE_NAME_PREFIX: &str = "mawaku";
pub const DEFAULT_RANDOM_SUFFIX_LENGTH: usize = 5;
pub const COMPONENT_MAX_LEN: usize = 10;
const SUFFIX_ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";

#[derive(Debug, Clone)]
pub struct ImageNameBuilder {
    parts: Vec<String>,
    random_suffix_length: usize,
}

impl ImageNameBuilder {
    pub fn new(prefix: impl Into<String>) -> Self {
        Self {
            parts: vec![prefix.into()],
            random_suffix_length: DEFAULT_RANDOM_SUFFIX_LENGTH,
        }
    }

    pub fn with_random_suffix_length(mut self, length: usize) -> Self {
        debug_assert!(length <= SUFFIX_ALPHABET.len());
        self.random_suffix_length = length;
        self
    }

    pub fn push_component(&mut self, value: Option<&str>) {
        if let Some(value) = value {
            if let Some(token) = component_token(value) {
                self.parts.push(token);
            }
        }
    }

    pub fn build(self) -> ImageNameContext {
        let base = self.parts.join("-");
        ImageNameContext {
            base,
            random_suffix_length: self.random_suffix_length,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ImageNameContext {
    base: String,
    random_suffix_length: usize,
}

impl ImageNameContext {
    pub fn new<'a>(
        prefix: impl Into<String>,
        components: impl IntoIterator<Item = Option<&'a str>>,
    ) -> Self {
        let mut builder = ImageNameBuilder::new(prefix);
        for component in components {
            builder.push_component(component);
        }
        builder.build()
    }

    pub fn file_stem(&self, index: usize) -> String {
        let suffix = unique_suffix(self.random_suffix_length);
        format!("{}-p{}-{}", self.base, index, suffix)
    }
}

pub fn component_token(input: &str) -> Option<String> {
    slugify(input).map(|slug| truncate_component(&slug))
}

pub fn slugify(input: &str) -> Option<String> {
    let mut slug = String::new();
    let mut last_was_separator = false;

    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
            last_was_separator = false;
        } else if matches!(ch, ' ' | '-' | '_' | '.' | '/' | '\\') {
            if !last_was_separator && !slug.is_empty() {
                slug.push('-');
                last_was_separator = true;
            }
        } else if !last_was_separator && !slug.is_empty() {
            slug.push('-');
            last_was_separator = true;
        }
    }

    let slug = slug.trim_matches('-').to_string();
    if slug.is_empty() { None } else { Some(slug) }
}

pub fn truncate_component(slug: &str) -> String {
    if slug.len() <= COMPONENT_MAX_LEN {
        return slug.to_string();
    }

    let truncated: String = slug.chars().take(COMPONENT_MAX_LEN).collect();
    let trimmed = truncated.trim_end_matches('-').to_string();
    if trimmed.is_empty() {
        truncated
    } else {
        trimmed
    }
}

fn unique_suffix(length: usize) -> String {
    debug_assert!(length <= SUFFIX_ALPHABET.len());
    let mut rng = thread_rng();
    SUFFIX_ALPHABET
        .choose_multiple(&mut rng, length)
        .copied()
        .map(char::from)
        .collect()
}

pub fn trimmed_or_none<'a>(input: Option<&'a str>) -> Option<&'a str> {
    input.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    })
}

pub fn list_or_unspecified<I, S>(items: I) -> String
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut filtered = Vec::new();

    for item in items {
        let trimmed = item.as_ref().trim();
        if !trimmed.is_empty() {
            filtered.push(trimmed.to_string());
        }
    }

    if filtered.is_empty() {
        "Unspecified".to_string()
    } else {
        filtered.join(", ")
    }
}

pub fn format_context_line(label: &str, value: Option<&str>) -> String {
    match trimmed_or_none(value) {
        Some(text) => format!("{label}: {text}"),
        None => format!("{label}: Unspecified"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn component_token_slugifies_input() {
        let token = component_token("Hakone, Japan");
        assert_eq!(token.as_deref(), Some("hakone-jap"));
    }

    #[test]
    fn slugify_preserves_alphanumeric_segments() {
        let slug = slugify("Hakone, Japan");
        assert_eq!(slug.as_deref(), Some("hakone-japan"));
    }

    #[test]
    fn builder_discards_empty_components() {
        let mut builder = ImageNameBuilder::new(DEFAULT_FILE_NAME_PREFIX);
        builder.push_component(Some("Hakone"));
        builder.push_component(Some("   "));
        builder.push_component(None);
        let context = builder.build();
        assert_eq!(context.base, "mawaku-hakone");
    }

    #[test]
    fn file_stem_includes_random_suffix() {
        let context = ImageNameBuilder::new(DEFAULT_FILE_NAME_PREFIX).build();
        let stem = context.file_stem(1);
        let (_, suffix) = stem
            .rsplit_once('-')
            .expect("file stem contains random suffix");
        assert_eq!(suffix.len(), DEFAULT_RANDOM_SUFFIX_LENGTH);
    }
}
