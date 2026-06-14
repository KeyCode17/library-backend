//! The exported engine: construct once on the mobile side, call `rank`.

use uuid::Uuid;

use recommender::{CandidateBook, Preferences, Recommender};

use crate::dto::{CandidateBookDto, PreferencesDto};
use crate::error::FfiError;

#[derive(uniffi::Object)]
pub struct RecommenderEngine {
    inner: Recommender,
}

impl Default for RecommenderEngine {
    fn default() -> Self {
        Self {
            inner: Recommender::new(),
        }
    }
}

#[uniffi::export]
impl RecommenderEngine {
    #[uniffi::constructor]
    pub fn new() -> Self {
        Self::default()
    }

    /// Rank `candidates` for `preferences`; returns candidate ids best-first.
    /// Errors only if a candidate id is not a valid uuid.
    pub fn rank(
        &self,
        preferences: PreferencesDto,
        candidates: Vec<CandidateBookDto>,
    ) -> Result<Vec<String>, FfiError> {
        let prefs = Preferences {
            preferred_shelves: preferences.preferred_shelves,
            preferred_authors: preferences.preferred_authors,
            available_only: preferences.available_only,
        };

        let mut domain_candidates = Vec::with_capacity(candidates.len());
        for candidate in candidates {
            let id = Uuid::parse_str(&candidate.id).map_err(|_| FfiError::Validation {
                msg: format!("invalid uuid: {}", candidate.id),
            })?;
            domain_candidates.push(CandidateBook {
                id,
                shelf: candidate.shelf,
                author: candidate.author,
                available: candidate.available,
            });
        }

        let ranked = self.inner.rank(&prefs, &domain_candidates);
        Ok(ranked.into_iter().map(|id| id.to_string()).collect())
    }
}
