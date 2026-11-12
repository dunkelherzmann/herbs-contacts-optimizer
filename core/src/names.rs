//! Working on the name fields on their own: flattening the parts into one full name, splitting a
//! full name back into parts, and spotting the merge artefact where the Name field already holds
//! the full name while the middle and last fields repeat it.
//!
//! The "Name" field is the contact's `first` field (the editor's "First name"): it is the one that
//! can hold a whole name. These functions only ever touch `first`, `middle` and `last`.

use crate::model::Contact;
use crate::text::plain;

/// Joins the first, middle and last names into one full name, with single spaces and no empty
/// parts: `("Herbert", "Henry", "Harrison")` → `"Herbert Henry Harrison"`.
pub fn flatten(first: &str, middle: &str, last: &str) -> String {
    [first, middle, last].into_iter().map(str::trim).filter(|part| !part.is_empty()).collect::<Vec<_>>().join(" ")
}

/// Splits a full name into first, middle and last. The first word is the first name, the last word
/// the last name, and whatever sits between is the middle name:
/// - `"Allan Michael Turing"` → `("Allan", "Michael", "Turing")`
/// - `"Allan Turing"` → `("Allan", "", "Turing")` (two words have no middle)
/// - `"Allan"` → `("Allan", "", "")`
/// - `""` → `("", "", "")`
///
/// It splits on spaces only, so real names it cannot know about (a particle like "van der", a
/// two-word surname, a suffix) come out wrong and want a look before they are kept.
pub fn split(full: &str) -> (String, String, String) {
    let words: Vec<&str> = full.split_whitespace().collect();
    match words.as_slice() {
        [] => (String::new(), String::new(), String::new()),
        [one] => (one.to_string(), String::new(), String::new()),
        [first, last] => (first.to_string(), String::new(), last.to_string()),
        [first, middle @ .., last] => (first.to_string(), middle.join(" "), last.to_string()),
    }
}

/// True when the Name field already holds the whole name and the middle and last fields merely
/// repeat its words — the artefact left when two copies of a contact are merged (Name
/// "Allan Michael Turing" with middle "Michael" and last "Turing").
///
/// It fires only when the parts are *redundant* with the Name field, never merely when both are
/// filled: a healthy contact keeps "Allan Turing" in its display name while first/last hold the
/// parts, and that is left alone.
pub fn redundant(first: &str, middle: &str, last: &str) -> bool {
    // The Name field must itself hold more than one word, and at least one part must be there to
    // repeat; otherwise there is nothing redundant.
    if first.split_whitespace().count() < 2 || (middle.trim().is_empty() && last.trim().is_empty()) {
        return false;
    }
    let (_, split_middle, split_last) = split(first);
    plain(&split_middle) == plain(middle) && plain(&split_last) == plain(last)
}

impl Contact {
    /// Moves the whole name into the Name field: first, middle and last become one line and the
    /// middle and last fields are emptied. Returns false when there was nothing to join.
    pub fn flatten_name(&mut self) -> bool {
        if self.middle.trim().is_empty() && self.last.trim().is_empty() {
            return false;
        }
        self.first = flatten(&self.first, &self.middle, &self.last);
        self.middle.clear();
        self.last.clear();
        true
    }

    /// Splits the Name field into first, middle and last. Returns false when the Name field holds
    /// at most one word, so there is nothing to split.
    pub fn split_name(&mut self) -> bool {
        if self.first.split_whitespace().count() < 2 {
            return false;
        }
        (self.first, self.middle, self.last) = split(&self.first);
        true
    }

    /// True when this contact's name parts are redundant with its Name field (see [`redundant`]).
    pub fn has_redundant_name(&self) -> bool {
        redundant(&self.first, &self.middle, &self.last)
    }

    /// Keeps the whole name in the Name field and empties the middle and last. For a contact whose
    /// Name field already holds the full name (`has_redundant_name`), this is the flatten fix: it
    /// drops the repeated parts without touching the Name field — unlike `flatten_name`, which
    /// joins the parts and so must not be used when they merely repeat the Name field.
    pub fn keep_full_name(&mut self) {
        self.middle.clear();
        self.last.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flatten_joins_the_parts() {
        assert_eq!(flatten("Herbert", "Henry", "Harrison"), "Herbert Henry Harrison");
        assert_eq!(flatten("Allan", "", "Turing"), "Allan Turing");
        assert_eq!(flatten("  Allan ", " ", " Turing "), "Allan Turing");
        assert_eq!(flatten("Allan", "", ""), "Allan");
        assert_eq!(flatten("", "", ""), "");
    }

    #[test]
    fn split_pulls_the_parts_apart() {
        assert_eq!(split("Allan Michael Turing"), ("Allan".into(), "Michael".into(), "Turing".into()));
        assert_eq!(split("Allan Turing"), ("Allan".into(), String::new(), "Turing".into()));
        assert_eq!(split("Allan"), ("Allan".into(), String::new(), String::new()));
        assert_eq!(split(""), (String::new(), String::new(), String::new()));
        assert_eq!(split("   "), (String::new(), String::new(), String::new()));
        // Extra spaces collapse.
        assert_eq!(split("Allan   Michael   Turing"), ("Allan".into(), "Michael".into(), "Turing".into()));
        // Four words: everything in the middle stays in the middle name.
        assert_eq!(split("John Fitzgerald Ronald Kennedy"), ("John".into(), "Fitzgerald Ronald".into(), "Kennedy".into()));
    }

    #[test]
    fn split_gets_hard_names_wrong_as_expected() {
        // A particle surname is split on the space like any other; the caller must correct it.
        assert_eq!(split("Vincent van Gogh"), ("Vincent".into(), "van".into(), "Gogh".into()));
        // A two-word surname loses its first word to the middle name.
        assert_eq!(split("Gabriel García Márquez"), ("Gabriel".into(), "García".into(), "Márquez".into()));
    }

    #[test]
    fn flatten_then_split_round_trips_a_plain_name() {
        let full = flatten("Allan", "Michael", "Turing");
        assert_eq!(split(&full), ("Allan".into(), "Michael".into(), "Turing".into()));
    }

    #[test]
    fn redundant_catches_the_merge_artefact() {
        assert!(redundant("Allan Michael Turing", "Michael", "Turing"));
        assert!(redundant("Allan Turing", "", "Turing"));
        // Case and accents do not matter when the parts are compared.
        assert!(redundant("Allan Michael Turing", "michael", "TURING"));
    }

    #[test]
    fn redundant_leaves_healthy_contacts_alone() {
        // A healthy contact: the parts are separate, the Name field holds only the first name.
        assert!(!redundant("Allan", "", "Turing"));
        assert!(!redundant("Allan", "Michael", "Turing"));
        // A genuine two-word first name with no parts is not redundant.
        assert!(!redundant("Mary Jane", "", ""));
        // The Name field holds a full name, but the parts do not match it — leave it to the editor.
        assert!(!redundant("Allan Michael Turing", "", "Watson"));
        // Only one word in the Name field: nothing is redundant.
        assert!(!redundant("Allan", "Michael", "Turing"));
    }

    #[test]
    fn contact_methods_flatten_and_split() {
        let mut c = Contact { first: "Herbert".into(), middle: "Henry".into(), last: "Harrison".into(), ..Contact::default() };
        assert!(c.flatten_name());
        assert_eq!((c.first.as_str(), c.middle.as_str(), c.last.as_str()), ("Herbert Henry Harrison", "", ""));
        // Nothing left to flatten the second time.
        assert!(!c.flatten_name());

        assert!(c.split_name());
        assert_eq!((c.first.as_str(), c.middle.as_str(), c.last.as_str()), ("Herbert", "Henry", "Harrison"));
        // A single-word Name does not split.
        let mut single = Contact { first: "Herbert".into(), ..Contact::default() };
        assert!(!single.split_name());
    }

    #[test]
    fn the_two_tidy_fixes_both_remove_the_redundancy() {
        // Keep the full name: the Name field is left whole, the repeated parts go.
        let mut keep = Contact { first: "Allan Michael Turing".into(), middle: "Michael".into(), last: "Turing".into(), ..Contact::default() };
        keep.keep_full_name();
        assert_eq!((keep.first.as_str(), keep.middle.as_str(), keep.last.as_str()), ("Allan Michael Turing", "", ""));
        assert!(!keep.has_redundant_name());

        // Split into parts: the Name field becomes the first name alone, the parts stay.
        let mut split = Contact { first: "Allan Michael Turing".into(), middle: "Michael".into(), last: "Turing".into(), ..Contact::default() };
        split.split_name();
        assert_eq!((split.first.as_str(), split.middle.as_str(), split.last.as_str()), ("Allan", "Michael", "Turing"));
        assert!(!split.has_redundant_name());
    }

    #[test]
    fn contact_spots_a_redundant_name() {
        let artefact = Contact { first: "Allan Michael Turing".into(), middle: "Michael".into(), last: "Turing".into(), ..Contact::default() };
        assert!(artefact.has_redundant_name());
        let healthy = Contact { first: "Allan".into(), last: "Turing".into(), display_name: "Allan Turing".into(), ..Contact::default() };
        assert!(!healthy.has_redundant_name());
    }
}
