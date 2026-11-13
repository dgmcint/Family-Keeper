//! The whole family file and the lookups everything else uses.

use super::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use unicode_normalization::UnicodeNormalization;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Database {
    #[serde(default)]
    pub people: BTreeMap<u32, Person>,
    #[serde(default)]
    pub families: BTreeMap<u32, Family>,
    #[serde(default)]
    pub sources: BTreeMap<u32, Source>,
    /// Records this program has no screen for (repositories, submitters, media...).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub other_records: Vec<Node>,
    /// The home person: where the file opens, first in exported files.
    #[serde(default)]
    pub home: Option<u32>,
}

pub(crate) fn fold(s: &str) -> String {
    s.nfd()
        .filter(|c| !unicode_normalization::char::is_combining_mark(*c))
        .flat_map(|c| c.to_lowercase())
        .collect()
}

/// Lowercases and drops accents, so "muller" finds "Müller".
pub fn search_fold(s: &str) -> String {
    fold(s)
}

impl Database {
    pub fn person(&self, id: u32) -> Option<&Person> {
        self.people.get(&id)
    }

    pub fn name_of(&self, id: u32) -> String {
        self.people.get(&id).map(|p| p.name()).unwrap_or_else(|| "(missing)".into())
    }

    fn next_person_id(&self) -> u32 {
        self.people.keys().next_back().map_or(1, |k| k + 1)
    }

    pub(crate) fn next_family_id(&self) -> u32 {
        self.families.keys().next_back().map_or(1, |k| k + 1)
    }

    pub fn next_source_id(&self) -> u32 {
        self.sources.keys().next_back().map_or(1, |k| k + 1)
    }

    pub fn add_person(&mut self, mut p: Person) -> u32 {
        let id = self.next_person_id();
        p.id = id;
        p.parent_families.clear();
        p.own_families.clear();
        self.people.insert(id, p);
        id
    }

    pub fn father_of(&self, id: u32) -> Option<u32> {
        self.parents_in_first_family(id).0
    }

    pub fn mother_of(&self, id: u32) -> Option<u32> {
        self.parents_in_first_family(id).1
    }

    fn parents_in_first_family(&self, id: u32) -> (Option<u32>, Option<u32>) {
        self.people
            .get(&id)
            .and_then(|p| p.parent_families.first())
            .and_then(|f| self.families.get(f))
            .map_or((None, None), |f| (f.husband, f.wife))
    }

    /// Everyone above this person, through every set of parents.
    pub fn ancestors(&self, id: u32) -> HashSet<u32> {
        let mut seen = HashSet::new();
        let mut todo = vec![id];
        while let Some(cur) = todo.pop() {
            let Some(p) = self.people.get(&cur) else { continue };
            for fam in p.parent_families.iter().filter_map(|f| self.families.get(f)) {
                for parent in [fam.husband, fam.wife].into_iter().flatten() {
                    if seen.insert(parent) {
                        todo.push(parent);
                    }
                }
            }
        }
        seen
    }

    /// How many people and families cite a source.
    pub fn source_use_count(&self, source: u32) -> usize {
        let count = |cites: &[Citation]| cites.iter().filter(|c| c.source == source).count();
        let events = |evs: &[Event]| evs.iter().map(|e| count(&e.citations)).sum::<usize>();
        self.people.values().map(|p| count(&p.citations) + events(&p.events)).sum::<usize>()
            + self.families.values().map(|f| count(&f.citations) + events(&f.events)).sum::<usize>()
    }

    /// Photo paths attached to a person, whether stored with them or as a shared media record.
    pub fn photos_of(&self, person: &Person) -> Vec<String> {
        let mut out = vec![];
        for n in person.extra.iter().filter(|n| n.tag == "OBJE") {
            let record = if n.value.starts_with('@') {
                self.other_records.iter().find(|r| {
                    r.tag == "OBJE" && r.xref.as_deref() == Some(n.value.trim_matches('@'))
                })
            } else {
                Some(n)
            };
            if let Some(file) = record.and_then(|r| r.child("FILE")) {
                if !file.value.trim().is_empty() {
                    out.push(file.value.clone());
                }
            }
        }
        out
    }
}

impl Database {
    /// The home person if they are still in the file, otherwise the first person.
    pub fn home_person(&self) -> Option<u32> {
        self.home.filter(|h| self.people.contains_key(h)).or_else(|| self.people.keys().next().copied())
    }

    /// The text kept for a person's number before renumbering ("REFN ... TYPE Old ID").
    pub fn old_id(p: &Person) -> Option<&str> {
        p.extra.iter().find(|n| n.tag == "REFN" && n.child("TYPE").is_some_and(|t| t.value.eq_ignore_ascii_case("Old ID"))).map(|n| n.value.as_str())
    }
}
