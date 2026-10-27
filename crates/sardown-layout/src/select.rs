//! Picking a subset of a laid-out document's pages, so one slide or chapter can be rendered on
//! its own while iterating.

use crate::LayoutOutput;
use std::str::FromStr;

/// A set of 1-based page numbers written as a comma-separated list of pages and ranges:
/// `3`, `2-5`, `-4` (up to page 4), `7-` (from page 7 on), `1,3,10-`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageSelection {
    /// Inclusive ranges; `None` as the end means "to the last page".
    ranges: Vec<(usize, Option<usize>)>,
}

impl PageSelection {
    /// Whether the 1-based `page` is selected.
    #[must_use]
    pub fn contains(&self, page: usize) -> bool {
        self.ranges.iter().any(|&(start, end)| page >= start && end.is_none_or(|end| page <= end))
    }
}

impl FromStr for PageSelection {
    type Err = String;

    fn from_str(spec: &str) -> Result<Self, String> {
        let number = |text: &str| -> Result<usize, String> {
            let n: usize = text.trim().parse().map_err(|_| format!("{:?} is not a page number", text.trim()))?;
            if n == 0 {
                return Err("pages are numbered from 1".to_string());
            }
            Ok(n)
        };
        let mut ranges = Vec::new();
        for part in spec.split(',') {
            let part = part.trim();
            if part.is_empty() {
                return Err("empty entry in the page list (expected e.g. `3`, `2-5`, `7-`)".to_string());
            }
            let range = match part.split_once('-') {
                None => {
                    let n = number(part)?;
                    (n, Some(n))
                }
                Some((start, end)) => {
                    let start = if start.trim().is_empty() { 1 } else { number(start)? };
                    let end = if end.trim().is_empty() { None } else { Some(number(end)?) };
                    if end.is_some_and(|end| end < start) {
                        return Err(format!("the range {part:?} ends before it starts"));
                    }
                    (start, end)
                }
            };
            ranges.push(range);
        }
        Ok(PageSelection { ranges })
    }
}

impl LayoutOutput {
    /// Keeps only the pages `selection` names. Text already placed on them (running headers and
    /// footers, so a footer still says `12 / 55`) is unchanged; links and outline entries that
    /// point at a dropped page are dropped, and the rest are renumbered to the new page order.
    ///
    /// Returns the kept pages' original 1-based numbers, in order.
    ///
    /// # Errors
    ///
    /// When the selection names no page of the document.
    pub fn retain_pages(&mut self, selection: &PageSelection) -> Result<Vec<usize>, String> {
        let total = self.pages.len();
        let kept: Vec<usize> = (0..total).filter(|&i| selection.contains(i + 1)).collect();
        if kept.is_empty() {
            return Err(format!("the page selection matches none of the document's {total} pages"));
        }
        let mut new_index = vec![None; total];
        for (new, &old) in kept.iter().enumerate() {
            new_index[old] = Some(new);
        }

        let mut position = 0;
        self.pages.retain(|_| {
            position += 1;
            new_index[position - 1].is_some()
        });
        let mut position = 0;
        self.page_contexts.retain(|_| {
            position += 1;
            new_index.get(position - 1).is_some_and(Option::is_some)
        });
        self.anchors.retain(|_, anchor| match new_index.get(anchor.page).copied().flatten() {
            Some(new) => {
                anchor.page = new;
                true
            }
            None => false,
        });
        let anchors = &self.anchors;
        self.toc_entries.retain(|entry| anchors.contains_key(&entry.id));
        Ok(kept.into_iter().map(|old| old + 1).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::PageSelection;

    fn picked(spec: &str, total: usize) -> Vec<usize> {
        let selection: PageSelection = spec.parse().unwrap();
        (1..=total).filter(|&p| selection.contains(p)).collect()
    }

    #[test]
    fn parses_pages_ranges_and_open_ends() {
        assert_eq!(picked("3", 10), [3]);
        assert_eq!(picked("2-4", 10), [2, 3, 4]);
        assert_eq!(picked("-3", 10), [1, 2, 3]);
        assert_eq!(picked("8-", 10), [8, 9, 10]);
        assert_eq!(picked("1, 3,7-8", 10), [1, 3, 7, 8]);
    }

    #[test]
    fn rejects_malformed_lists() {
        for bad in ["", "0", "a", "5-2", "1,,2", "1-x"] {
            assert!(bad.parse::<PageSelection>().is_err(), "{bad:?} should be rejected");
        }
    }
}
