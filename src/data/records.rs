//! The records a family file holds.

use super::*;
use serde::{Deserialize, Serialize};

/// A piece of GEDCOM this program has no field for. Kept so nothing from an
/// imported file gets lost when it's saved or exported again.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Node {
    pub tag: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub xref: Option<String>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub value: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<Node>,
}

impl Node {
    pub fn new(tag: &str, value: &str) -> Self {
        Node { tag: tag.into(), xref: None, value: value.into(), children: vec![] }
    }
    pub fn child(&self, tag: &str) -> Option<&Node> {
        self.children.iter().find(|c| c.tag == tag)
    }
    /// Calls `f` on this node and everything under it.
    pub fn walk_mut(&mut self, f: &mut impl FnMut(&mut Node)) {
        f(self);
        for c in &mut self.children {
            c.walk_mut(f);
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Sex {
    Male,
    Female,
    #[default]
    Unknown,
}

impl Sex {
    pub fn label(self) -> &'static str {
        match self {
            Sex::Male => "Male",
            Sex::Female => "Female",
            Sex::Unknown => "Unknown",
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Citation {
    pub source: u32,
    #[serde(default)]
    pub page: String,
    #[serde(default)]
    pub text: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extra: Vec<Node>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Event {
    pub tag: String,
    /// The value on the line itself: "Farmer" for an occupation, "Y" for a death with no details.
    #[serde(default)]
    pub value: String,
    /// The event's own wording, used by custom events.
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub date: String,
    #[serde(default)]
    pub place: String,
    #[serde(default)]
    pub temple: String,
    #[serde(default)]
    pub note: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub citations: Vec<Citation>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub date_extra: Vec<Node>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub place_extra: Vec<Node>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extra: Vec<Node>,
}

impl Event {
    pub fn new(tag: &str) -> Self {
        Event { tag: tag.into(), ..Default::default() }
    }
    pub fn is_blank(&self) -> bool {
        self.value.trim().is_empty()
            && self.kind.trim().is_empty()
            && self.date.trim().is_empty()
            && self.place.trim().is_empty()
            && self.temple.trim().is_empty()
            && self.note.trim().is_empty()
            && self.citations.is_empty()
            && self.date_extra.is_empty()
            && self.place_extra.is_empty()
            && self.extra.is_empty()
    }
    pub fn label(&self) -> String {
        if self.tag == "EVEN" || self.tag == "FACT" {
            if !self.kind.trim().is_empty() {
                return self.kind.trim().to_string();
            }
        }
        event_label(&self.tag).to_string()
    }
    /// "12 Mar 1850, Oxford" with whatever parts are filled in.
    pub fn summary(&self) -> String {
        let mut parts: Vec<&str> = vec![];
        for s in [&self.value, &self.date, &self.place, &self.temple] {
            let s = s.trim();
            if !s.is_empty() && !(s == "Y" && self.value.trim() == "Y") {
                parts.push(s);
            }
        }
        parts.join(", ")
    }
}

/// The year in a date such as "ABT 12 MAR 1850" or "1850/51", if there is one.
pub fn date_year(date: &str) -> Option<i32> {
    let mut digits = String::new();
    let mut found = None;
    for c in date.chars().chain(std::iter::once(' ')) {
        if c.is_ascii_digit() {
            digits.push(c);
        } else {
            if (3..=4).contains(&digits.len()) {
                found = digits.parse().ok();
                break;
            }
            digits.clear();
        }
    }
    found
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Person {
    pub id: u32,
    #[serde(default)]
    pub given: String,
    #[serde(default)]
    pub surname: String,
    #[serde(default)]
    pub suffix: String,
    #[serde(default)]
    pub sex: Sex,
    #[serde(default)]
    pub events: Vec<Event>,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub citations: Vec<Citation>,
    /// The families this person is a child in; the first is the one shown by default.
    #[serde(default)]
    pub parent_families: Vec<u32>,
    /// The families this person is a partner in.
    #[serde(default)]
    pub own_families: Vec<u32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub name_extra: Vec<Node>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extra: Vec<Node>,
}

impl Person {
    pub fn name(&self) -> String {
        let mut s = String::new();
        for part in [&self.given, &self.surname, &self.suffix] {
            let part = part.trim();
            if !part.is_empty() {
                if !s.is_empty() {
                    s.push(' ');
                }
                s.push_str(part);
            }
        }
        if s.is_empty() { "(no name)".to_string() } else { s }
    }
    /// "Alder, Thomas" for lists.
    pub fn list_name(&self) -> String {
        let (g, n) = (self.given.trim(), self.surname.trim());
        let mut s = match (n.is_empty(), g.is_empty()) {
            (false, false) => format!("{n}, {g}"),
            (false, true) => format!("{n}, (no given name)"),
            (true, false) => format!("(no surname), {g}"),
            (true, true) => "(no name)".to_string(),
        };
        if !self.suffix.trim().is_empty() {
            s.push(' ');
            s.push_str(self.suffix.trim());
        }
        s
    }
    pub fn event(&self, tag: &str) -> Option<&Event> {
        self.events.iter().find(|e| e.tag == tag)
    }
    pub fn date_of(&self, tag: &str) -> &str {
        self.event(tag).map(|e| e.date.as_str()).unwrap_or("")
    }
    pub fn place_of(&self, tag: &str) -> &str {
        self.event(tag).map(|e| e.place.as_str()).unwrap_or("")
    }
    pub fn birth_year(&self) -> Option<i32> {
        date_year(self.date_of("BIRT")).or_else(|| date_year(self.date_of("CHR")))
    }
    pub fn death_year(&self) -> Option<i32> {
        date_year(self.date_of("DEAT")).or_else(|| date_year(self.date_of("BURI")))
    }
    /// "1850–1921", "b. 1850", or "".
    pub fn lifespan(&self) -> String {
        match (self.birth_year(), self.death_year()) {
            (Some(b), Some(d)) => format!("{b}–{d}"),
            (Some(b), None) => format!("b. {b}"),
            (None, Some(d)) => format!("d. {d}"),
            (None, None) => String::new(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Family {
    pub id: u32,
    #[serde(default)]
    pub husband: Option<u32>,
    #[serde(default)]
    pub wife: Option<u32>,
    #[serde(default)]
    pub children: Vec<u32>,
    #[serde(default)]
    pub events: Vec<Event>,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub citations: Vec<Citation>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extra: Vec<Node>,
}

impl Family {
    pub fn event(&self, tag: &str) -> Option<&Event> {
        self.events.iter().find(|e| e.tag == tag)
    }
    pub fn partner_of(&self, person: u32) -> Option<u32> {
        if self.husband == Some(person) {
            self.wife
        } else if self.wife == Some(person) {
            self.husband
        } else {
            None
        }
    }
    pub(crate) fn has_details(&self) -> bool {
        !self.events.is_empty()
            || !self.notes.trim().is_empty()
            || !self.citations.is_empty()
            || !self.extra.is_empty()
    }
    pub(crate) fn member_count(&self) -> usize {
        self.husband.is_some() as usize + self.wife.is_some() as usize + self.children.len()
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Source {
    pub id: u32,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub publication: String,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub notes: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extra: Vec<Node>,
}
