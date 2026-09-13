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

#[test]
fn reference_distribution_handles_sparse_and_duplicate_details() {
    let details = [" cedar ", "", "CEDAR", "linen"].map(String::from);
    assert_eq!(distribute_prompt_details(&details, 0, 3, 2), vec!["cedar"]);
    assert_eq!(distribute_prompt_details(&details, 1, 3, 2), vec!["linen"]);
    assert_eq!(distribute_prompt_details(&details, 2, 3, 2), vec!["cedar"]);
    assert!(distribute_prompt_details(&[], 0, 3, 2).is_empty());
}
