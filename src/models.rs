//! Core domain types for LearnSwap.
//!
//! A member offers some skills (`teaching`) and wants some others (`learning`).
//! A *swap* is a pair of members where each side can teach something the other
//! side wants to learn -- that mutual requirement is what separates LearnSwap
//! from a one-way tutoring listing.

use serde::Serialize;
use uuid::Uuid;

/// A skill tag. `key` is the normalized form used for comparisons so that
/// "Python", "python " and "PYTHON" all match each other.
#[derive(Debug, Clone, Serialize)]
pub struct Skill {
    pub label: String,
    pub key: String,
}

impl Skill {
    pub fn new(label: &str) -> Self {
        let label = label.trim();
        Self {
            label: label.to_string(),
            key: label.to_lowercase(),
        }
    }

    /// Parses a comma-separated field from a form into skill tags, dropping
    /// blanks and duplicates.
    pub fn parse_list(raw: &str) -> Vec<Skill> {
        let mut out: Vec<Skill> = Vec::new();
        for part in raw.split(',') {
            let skill = Skill::new(part);
            if skill.key.is_empty() || out.iter().any(|s| s.key == skill.key) {
                continue;
            }
            out.push(skill);
        }
        out
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Member {
    pub id: Uuid,
    pub name: String,
    pub headline: String,
    pub bio: String,
    pub teaching: Vec<Skill>,
    pub learning: Vec<Skill>,
}

impl Member {
    pub fn new(name: &str, headline: &str, bio: &str, teaching: &str, learning: &str) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.trim().to_string(),
            headline: headline.trim().to_string(),
            bio: bio.trim().to_string(),
            teaching: Skill::parse_list(teaching),
            learning: Skill::parse_list(learning),
        }
    }

    /// Skills this member can teach that `other` wants to learn.
    fn can_teach(&self, other: &Member) -> Vec<Skill> {
        self.teaching
            .iter()
            .filter(|s| other.learning.iter().any(|w| w.key == s.key))
            .cloned()
            .collect()
    }

    /// True if any field contains `needle` (already lowercased by the caller).
    pub fn matches_query(&self, needle: &str) -> bool {
        if needle.is_empty() {
            return true;
        }
        self.name.to_lowercase().contains(needle)
            || self.headline.to_lowercase().contains(needle)
            || self
                .teaching
                .iter()
                .chain(&self.learning)
                .any(|s| s.key.contains(needle))
    }
}

/// A viable two-way exchange between the member being viewed and a candidate.
#[derive(Debug, Clone, Serialize)]
pub struct Swap {
    pub member: Member,
    /// What the candidate would teach the viewer.
    pub they_teach: Vec<Skill>,
    /// What the viewer would teach the candidate.
    pub you_teach: Vec<Skill>,
}

impl Swap {
    /// Builds a swap only if the exchange works in *both* directions.
    pub fn between(viewer: &Member, candidate: &Member) -> Option<Swap> {
        if viewer.id == candidate.id {
            return None;
        }
        let they_teach = candidate.can_teach(viewer);
        let you_teach = viewer.can_teach(candidate);
        if they_teach.is_empty() || you_teach.is_empty() {
            return None;
        }
        Some(Swap {
            member: candidate.clone(),
            they_teach,
            you_teach,
        })
    }

    /// Ranking score: how much the two members have to trade.
    pub fn strength(&self) -> usize {
        self.they_teach.len() + self.you_teach.len()
    }
}
