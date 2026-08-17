use std::collections::HashMap;

pub struct SlugGenerator {
    seen: HashMap<String, u32>,
}

impl SlugGenerator {
    pub fn new() -> Self {
        Self { seen: HashMap::new() }
    }

    pub fn generate(&mut self, heading_text: &str) -> String {
        let base = Self::slugify(heading_text);
        let count = self.seen.entry(base.clone()).or_insert(0);
        let slug = if *count == 0 { base.clone() } else { format!("{}-{}", base, count) };
        *count += 1;
        slug
    }

    fn slugify(text: &str) -> String {
        let mut slug = String::with_capacity(text.len());
        let mut last_was_dash = true; // suppress leading dashes
        for ch in text.chars() {
            if ch.is_alphanumeric() {
                slug.extend(ch.to_lowercase());
                last_was_dash = false;
            } else if !last_was_dash {
                slug.push('-');
                last_was_dash = true;
            }
        }
        while slug.ends_with('-') {
            slug.pop();
        }
        slug
    }
}

impl Default for SlugGenerator {
    fn default() -> Self {
        Self::new()
    }
}

// convenience free function used directly by tests/callers that don't need
// cross-document collision tracking
pub fn generate_heading_id(text: &str) -> String {
    SlugGenerator::slugify(text)
}
