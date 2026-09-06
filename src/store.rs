//! In-memory member store.
//!
//! This is deliberately the only place that knows how members are persisted.
//! Swapping in a real database later means reimplementing these methods; no
//! route handler touches storage directly.

use std::sync::RwLock;

use uuid::Uuid;

use crate::models::{Member, Swap};

#[derive(Debug, Default)]
pub struct Store {
    members: RwLock<Vec<Member>>,
}

impl Store {
    pub fn new() -> Self {
        Self::default()
    }

    /// A store preloaded with sample members so the app has something to show
    /// on a fresh clone.
    pub fn with_seed_data() -> Self {
        let store = Store::new();
        for member in seed_members() {
            store.insert(member);
        }
        store
    }

    fn read(&self) -> std::sync::RwLockReadGuard<'_, Vec<Member>> {
        // A poisoned lock means another thread panicked mid-write. The data is
        // still structurally valid, so recover rather than cascade the panic.
        self.members.read().unwrap_or_else(|e| e.into_inner())
    }

    pub fn insert(&self, member: Member) -> Uuid {
        let id = member.id;
        let mut members = self.members.write().unwrap_or_else(|e| e.into_inner());
        members.push(member);
        id
    }

    pub fn count(&self) -> usize {
        self.read().len()
    }

    pub fn all(&self) -> Vec<Member> {
        self.read().clone()
    }

    pub fn get(&self, id: Uuid) -> Option<Member> {
        self.read().iter().find(|m| m.id == id).cloned()
    }

    /// Free-text search over names, headlines and skill tags.
    pub fn search(&self, query: &str) -> Vec<Member> {
        let needle = query.trim().to_lowercase();
        self.read()
            .iter()
            .filter(|m| m.matches_query(&needle))
            .cloned()
            .collect()
    }

    /// Every member who can trade skills with `id`, strongest match first.
    pub fn swaps_for(&self, id: Uuid) -> Vec<Swap> {
        let members = self.read();
        let Some(viewer) = members.iter().find(|m| m.id == id) else {
            return Vec::new();
        };
        let mut swaps: Vec<Swap> = members
            .iter()
            .filter_map(|candidate| Swap::between(viewer, candidate))
            .collect();
        swaps.sort_by(|a, b| {
            b.strength()
                .cmp(&a.strength())
                .then_with(|| a.member.name.cmp(&b.member.name))
        });
        swaps
    }

    /// Number of distinct skills anyone has offered or asked for. Drives the
    /// counter on the landing page.
    pub fn distinct_skill_count(&self) -> usize {
        let members = self.read();
        let mut keys: Vec<&str> = members
            .iter()
            .flat_map(|m| m.teaching.iter().chain(&m.learning))
            .map(|s| s.key.as_str())
            .collect();
        keys.sort_unstable();
        keys.dedup();
        keys.len()
    }

    /// The strongest swaps anywhere in the community, used as a landing-page
    /// showcase. Each pair appears once.
    pub fn featured_swaps(&self, limit: usize) -> Vec<(Member, Swap)> {
        let members = self.read();
        let mut pairs: Vec<(Member, Swap)> = Vec::new();
        for (i, viewer) in members.iter().enumerate() {
            for candidate in members.iter().skip(i + 1) {
                if let Some(swap) = Swap::between(viewer, candidate) {
                    pairs.push((viewer.clone(), swap));
                }
            }
        }
        pairs.sort_by(|a, b| b.1.strength().cmp(&a.1.strength()));
        pairs.truncate(limit);
        pairs
    }
}

fn seed_members() -> Vec<Member> {
    vec![
        Member::new(
            "Ana Ruiz",
            "CS junior, weekend ceramicist",
            "Happy to walk through data structures homework. I learn best by \
             explaining things out loud, so teaching is genuinely useful to me too.",
            "Rust, Data Structures, Pottery",
            "Spanish, Guitar",
        ),
        Member::new(
            "Marcus Bell",
            "Music ed major",
            "I have taught guitar since high school. Trying to get comfortable \
             enough with code to build a practice-tracking app.",
            "Guitar, Music Theory",
            "Rust, Web Development",
        ),
        Member::new(
            "Priya Nair",
            "Bilingual, learning to cook properly",
            "Native Spanish and Hindi speaker. I can do conversation practice \
             any weekday evening.",
            "Spanish, Hindi",
            "Baking, Data Structures",
        ),
        Member::new(
            "Tom Okafor",
            "Line cook turned CS student",
            "Six years in restaurant kitchens. Ask me about bread.",
            "Baking, Knife Skills",
            "Pottery, Music Theory",
        ),
        Member::new(
            "Lena Fischer",
            "Front-end dev, absolute beginner at pottery",
            "I can get you from zero to a deployed web page in an afternoon.",
            "Web Development, CSS",
            "Pottery, Knife Skills",
        ),
    ]
}
