//! The decision-tree ranking (PRD §3: "recommendations via a decision tree").
//!
//! Each candidate is scored by walking a small tree of rules; candidates are then
//! ordered by score (descending), ties broken by id (ascending) so the output is
//! fully deterministic.

use uuid::Uuid;

use crate::domain::{CandidateBook, Preferences};

/// Points each rule contributes. Shelf match dominates author match, which
/// dominates a small nudge for being on the shelf (available).
#[derive(Debug, Clone, Copy)]
pub struct Weights {
    pub shelf_match: i64,
    pub author_match: i64,
    pub available: i64,
}

impl Default for Weights {
    fn default() -> Self {
        Self {
            shelf_match: 10,
            author_match: 5,
            available: 1,
        }
    }
}

/// The ranking engine. Stateless apart from its scoring weights, so it is cheap
/// to build and trivially shareable.
#[derive(Debug, Clone, Default)]
pub struct Recommender {
    weights: Weights,
}

impl Recommender {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_weights(weights: Weights) -> Self {
        Self { weights }
    }

    /// Rank `candidates` for `preferences`, returning ids best-first.
    ///
    /// `available_only` filters first; remaining candidates are scored and sorted
    /// (score desc, then id asc for determinism).
    pub fn rank(&self, preferences: &Preferences, candidates: &[CandidateBook]) -> Vec<Uuid> {
        let mut scored: Vec<(i64, Uuid)> = candidates
            .iter()
            .filter(|book| !preferences.available_only || book.available)
            .map(|book| (self.score(preferences, book), book.id))
            .collect();

        // Higher score first; break ties by id so results are stable.
        scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
        scored.into_iter().map(|(_, id)| id).collect()
    }

    /// The decision tree: sum the points for each matched rule.
    fn score(&self, preferences: &Preferences, book: &CandidateBook) -> i64 {
        let mut score = 0;

        if contains_ignore_case(&preferences.preferred_shelves, &book.shelf) {
            score += self.weights.shelf_match;
        }
        if contains_ignore_case(&preferences.preferred_authors, &book.author) {
            score += self.weights.author_match;
        }
        if book.available {
            score += self.weights.available;
        }

        score
    }
}

fn contains_ignore_case(haystack: &[String], needle: &str) -> bool {
    haystack
        .iter()
        .any(|candidate| candidate.eq_ignore_ascii_case(needle))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn book(id: u128, shelf: &str, author: &str, available: bool) -> CandidateBook {
        CandidateBook {
            id: Uuid::from_u128(id),
            shelf: shelf.to_owned(),
            author: author.to_owned(),
            available,
        }
    }

    #[test]
    fn shelf_match_outranks_author_match_outranks_neither() {
        let prefs = Preferences {
            preferred_shelves: vec!["Tech".to_owned()],
            preferred_authors: vec!["Orwell".to_owned()],
            available_only: false,
        };
        let shelf_hit = book(1, "Tech", "Nobody", true);
        let author_hit = book(2, "Fiction", "Orwell", true);
        let no_hit = book(3, "Fiction", "Nobody", true);

        let ranked = Recommender::new().rank(&prefs, &[no_hit, author_hit, shelf_hit]);
        assert_eq!(
            ranked,
            vec![Uuid::from_u128(1), Uuid::from_u128(2), Uuid::from_u128(3)]
        );
    }

    #[test]
    fn matching_is_case_insensitive() {
        let prefs = Preferences {
            preferred_shelves: vec!["tech".to_owned()],
            preferred_authors: vec![],
            available_only: false,
        };
        let ranked = Recommender::new().rank(&prefs, &[book(7, "TECH", "x", true)]);
        assert_eq!(ranked, vec![Uuid::from_u128(7)]);
    }

    #[test]
    fn available_only_filters_unavailable_books() {
        let prefs = Preferences {
            preferred_shelves: vec![],
            preferred_authors: vec![],
            available_only: true,
        };
        let ranked = Recommender::new().rank(
            &prefs,
            &[book(1, "Tech", "a", false), book(2, "Tech", "a", true)],
        );
        assert_eq!(ranked, vec![Uuid::from_u128(2)]);
    }

    #[test]
    fn equal_scores_break_ties_by_id_for_determinism() {
        let prefs = Preferences::default();
        let ranked = Recommender::new().rank(
            &prefs,
            &[
                book(3, "x", "y", true),
                book(1, "x", "y", true),
                book(2, "x", "y", true),
            ],
        );
        assert_eq!(
            ranked,
            vec![Uuid::from_u128(1), Uuid::from_u128(2), Uuid::from_u128(3)]
        );
    }
}
