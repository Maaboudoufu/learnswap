//! Core domain types for LearnSwap.
//!
//! A user offers some skills (`teaching`) and wants some others (`learning`).
//! A *swap* is a pair of users where each side can teach something the other
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

    /// Renders tags back to the comma-separated form stored in the database.
    pub fn join(skills: &[Skill]) -> String {
        skills
            .iter()
            .map(|s| s.label.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    }
}

/// A member of the site. Deliberately carries no credentials: this is the type
/// that gets handed to templates, and a password hash must never end up in a
/// rendered page. The hash stays inside `store`.
#[derive(Debug, Clone, Serialize)]
pub struct User {
    pub id: Uuid,
    /// Not serialized -- an email address is contact information, and nothing
    /// public should leak it just because a template interpolated a whole user.
    #[serde(skip_serializing)]
    pub email: String,
    pub name: String,
    pub headline: String,
    pub bio: String,
    pub teaching: Vec<Skill>,
    pub learning: Vec<Skill>,
}

impl User {
    /// Skills this user can teach that `other` wants to learn.
    fn can_teach(&self, other: &User) -> Vec<Skill> {
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

/// A viable two-way exchange between the user being viewed and a candidate.
#[derive(Debug, Clone, Serialize)]
pub struct Swap {
    pub member: User,
    /// What the candidate would teach the viewer.
    pub they_teach: Vec<Skill>,
    /// What the viewer would teach the candidate.
    pub you_teach: Vec<Skill>,
}

impl Swap {
    /// Builds a swap only if the exchange works in *both* directions.
    pub fn between(viewer: &User, candidate: &User) -> Option<Swap> {
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

    /// Ranking score: how much the two users have to trade.
    pub fn strength(&self) -> usize {
        self.they_teach.len() + self.you_teach.len()
    }
}
