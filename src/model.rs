//! The family records: people, families, sources, and everything that links them.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use unicode_normalization::UnicodeNormalization;

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

/// Person events and facts, in the order they're offered.
pub const PERSON_EVENTS: &[&str] = &[
    "BIRT", "CHR", "DEAT", "BURI", "CREM", "ADOP", "BAPM", "BARM", "BASM", "BLES", "CHRA", "CONF",
    "FCOM", "ORDN", "NATU", "EMIG", "IMMI", "CENS", "PROB", "WILL", "GRAD", "RETI", "RESI", "OCCU",
    "EDUC", "RELI", "TITL", "NATI", "CAST", "DSCR", "IDNO", "PROP", "SSN", "EVEN", "FACT", "BAPL",
    "CONL", "ENDL", "SLGC",
];
/// Family events, in the order they're offered.
pub const FAMILY_EVENTS: &[&str] = &[
    "MARR", "ENGA", "MARB", "MARC", "MARL", "MARS", "DIV", "DIVF", "ANUL", "CENS", "RESI", "EVEN",
    "SLGS",
];

pub fn event_label(tag: &str) -> &str {
    match tag {
        "BIRT" => "Birth",
        "CHR" => "Christening",
        "DEAT" => "Death",
        "BURI" => "Burial",
        "CREM" => "Cremation",
        "ADOP" => "Adoption",
        "BAPM" => "Baptism",
        "BARM" => "Bar mitzvah",
        "BASM" => "Bat mitzvah",
        "BLES" => "Blessing",
        "CHRA" => "Adult christening",
        "CONF" => "Confirmation",
        "FCOM" => "First communion",
        "ORDN" => "Ordination",
        "NATU" => "Naturalization",
        "EMIG" => "Emigration",
        "IMMI" => "Immigration",
        "CENS" => "Census",
        "PROB" => "Probate",
        "WILL" => "Will",
        "GRAD" => "Graduation",
        "RETI" => "Retirement",
        "RESI" => "Residence",
        "OCCU" => "Occupation",
        "EDUC" => "Education",
        "RELI" => "Religion",
        "TITL" => "Title",
        "NATI" => "Nationality",
        "CAST" => "Caste",
        "DSCR" => "Description",
        "IDNO" => "ID number",
        "PROP" => "Property",
        "SSN" => "Social Security number",
        "EVEN" => "Other event",
        "FACT" => "Other fact",
        "BAPL" => "Baptism (LDS)",
        "CONL" => "Confirmation (LDS)",
        "ENDL" => "Endowment (LDS)",
        "SLGC" => "Sealing to parents (LDS)",
        "SLGS" => "Sealing to spouse (LDS)",
        "MARR" => "Marriage",
        "ENGA" => "Engagement",
        "MARB" => "Marriage banns",
        "MARC" => "Marriage contract",
        "MARL" => "Marriage license",
        "MARS" => "Marriage settlement",
        "DIV" => "Divorce",
        "DIVF" => "Divorce filed",
        "ANUL" => "Annulment",
        other => other,
    }
}

pub fn is_lds_event(tag: &str) -> bool {
    matches!(tag, "BAPL" | "CONL" | "ENDL" | "SLGC" | "SLGS")
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
    fn has_details(&self) -> bool {
        !self.events.is_empty()
            || !self.notes.trim().is_empty()
            || !self.citations.is_empty()
            || !self.extra.is_empty()
    }
    fn member_count(&self) -> usize {
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

fn fold(s: &str) -> String {
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
    fn next_family_id(&self) -> u32 {
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

    fn check_parent_child(&self, parent: u32, child: u32) -> Result<(), String> {
        if parent == child {
            return Err("A person can't be their own parent.".into());
        }
        if self.ancestors(parent).contains(&child) {
            return Err(format!(
                "{} is already an ancestor of {}, so can't also be their child.",
                self.name_of(child),
                self.name_of(parent)
            ));
        }
        Ok(())
    }

    fn new_family(&mut self) -> u32 {
        let id = self.next_family_id();
        self.families.insert(id, Family { id, ..Default::default() });
        id
    }

    fn link_child(&mut self, family: u32, child: u32) {
        if let Some(f) = self.families.get_mut(&family) {
            if !f.children.contains(&child) {
                f.children.push(child);
            }
        }
        if let Some(p) = self.people.get_mut(&child) {
            if !p.parent_families.contains(&family) {
                p.parent_families.push(family);
            }
        }
    }

    fn set_partner(&mut self, family: u32, person: u32, as_husband: bool) {
        if let Some(f) = self.families.get_mut(&family) {
            if as_husband {
                f.husband = Some(person);
            } else {
                f.wife = Some(person);
            }
        }
        if let Some(p) = self.people.get_mut(&person) {
            if !p.own_families.contains(&family) {
                p.own_families.push(family);
            }
        }
    }

    /// Adds `parent` above `child`, filling the empty slot in their first set of parents.
    pub fn add_parent(&mut self, child: u32, parent: u32, as_father: bool) -> Result<(), String> {
        self.check_parent_child(parent, child)?;
        let existing = self.people.get(&child).and_then(|p| p.parent_families.first().copied());
        let family = match existing {
            Some(f) => {
                let fam = &self.families[&f];
                let slot = if as_father { fam.husband } else { fam.wife };
                if slot.is_some() {
                    return Err("That parent is already filled in.".into());
                }
                if fam.children.iter().any(|c| *c == parent) {
                    return Err("That person is listed as a sibling in this family.".into());
                }
                f
            }
            None => {
                let f = self.new_family();
                self.link_child(f, child);
                f
            }
        };
        self.set_partner(family, parent, as_father);
        Ok(())
    }

    /// Makes a family for two partners. Returns the family.
    pub fn add_spouse(&mut self, person: u32, spouse: u32) -> Result<u32, String> {
        if person == spouse {
            return Err("A person can't be their own spouse.".into());
        }
        let Some(p) = self.people.get(&person) else { return Err("That person is gone.".into()) };
        let psex = p.sex;
        let own = p.own_families.clone();
        for f in own.iter().filter_map(|f| self.families.get(f)) {
            if f.partner_of(person) == Some(spouse) {
                return Err("They are already listed as a couple.".into());
            }
        }
        // Use a family that has this person and no partner yet, if there is one.
        let open = own.iter().copied().find(|f| {
            self.families.get(f).is_some_and(|fam| fam.husband.is_none() || fam.wife.is_none())
        });
        let family = open.unwrap_or_else(|| self.new_family());
        let fam = &self.families[&family];
        if fam.children.contains(&spouse) {
            return Err("That person is a child in this family.".into());
        }
        let person_is_husband = match (fam.husband, fam.wife) {
            (Some(h), _) if h == person => true,
            (_, Some(w)) if w == person => false,
            _ => psex != Sex::Female,
        };
        self.set_partner(family, person, person_is_husband);
        self.set_partner(family, spouse, !person_is_husband);
        Ok(family)
    }

    /// Adds a child to a family, or to a new family with `person` as the only parent.
    pub fn add_child(&mut self, person: u32, family: Option<u32>, child: u32) -> Result<u32, String> {
        self.check_parent_child(person, child)?;
        if let Some(f) = family.and_then(|f| self.families.get(&f)) {
            if let Some(other) = f.partner_of(person) {
                self.check_parent_child(other, child)?;
            }
        }
        let family = match family {
            Some(f) => f,
            None => {
                let sex = self.people.get(&person).map_or(Sex::Unknown, |p| p.sex);
                let f = self.new_family();
                self.set_partner(f, person, sex != Sex::Female);
                f
            }
        };
        self.link_child(family, child);
        Ok(family)
    }

    /// Adds a brother or sister in this person's first set of parents.
    pub fn add_sibling(&mut self, person: u32, sibling: u32) -> Result<(), String> {
        if person == sibling {
            return Err("A person can't be their own sibling.".into());
        }
        let existing = self.people.get(&person).and_then(|p| p.parent_families.first().copied());
        let family = match existing {
            Some(f) => {
                let fam = &self.families[&f];
                for parent in [fam.husband, fam.wife].into_iter().flatten() {
                    self.check_parent_child(parent, sibling)?;
                }
                f
            }
            None => {
                let f = self.new_family();
                self.link_child(f, person);
                f
            }
        };
        self.link_child(family, sibling);
        Ok(())
    }

    pub fn remove_child(&mut self, family: u32, child: u32) {
        if let Some(f) = self.families.get_mut(&family) {
            f.children.retain(|c| *c != child);
        }
        if let Some(p) = self.people.get_mut(&child) {
            p.parent_families.retain(|f| *f != family);
        }
        self.prune_family(family);
    }

    pub fn remove_partner(&mut self, family: u32, person: u32) {
        if let Some(f) = self.families.get_mut(&family) {
            if f.husband == Some(person) {
                f.husband = None;
            }
            if f.wife == Some(person) {
                f.wife = None;
            }
        }
        if let Some(p) = self.people.get_mut(&person) {
            p.own_families.retain(|f| *f != family);
        }
        self.prune_family(family);
    }

    /// Drops a family nobody is in any more (one person alone, with no details, isn't a family either).
    fn prune_family(&mut self, family: u32) {
        let Some(f) = self.families.get(&family) else { return };
        if f.member_count() > 1 || f.has_details() {
            return;
        }
        let f = self.families.remove(&family).expect("checked above");
        for id in [f.husband, f.wife].into_iter().flatten() {
            if let Some(p) = self.people.get_mut(&id) {
                p.own_families.retain(|x| *x != family);
            }
        }
        for id in f.children {
            if let Some(p) = self.people.get_mut(&id) {
                p.parent_families.retain(|x| *x != family);
            }
        }
    }

    fn for_each_extra(&mut self, f: &mut impl FnMut(&mut Vec<Node>)) {
        fn event(e: &mut Event, f: &mut impl FnMut(&mut Vec<Node>)) {
            f(&mut e.extra);
            for c in &mut e.citations {
                f(&mut c.extra);
            }
        }
        for p in self.people.values_mut() {
            f(&mut p.extra);
            f(&mut p.name_extra);
            p.events.iter_mut().for_each(|e| event(e, f));
            p.citations.iter_mut().for_each(|c| f(&mut c.extra));
        }
        for x in self.families.values_mut() {
            f(&mut x.extra);
            x.events.iter_mut().for_each(|e| event(e, f));
            x.citations.iter_mut().for_each(|c| f(&mut c.extra));
        }
        for s in self.sources.values_mut() {
            f(&mut s.extra);
        }
        f(&mut self.other_records);
    }

    pub fn delete_person(&mut self, id: u32) {
        let Some(p) = self.people.get(&id).cloned() else { return };
        for f in &p.own_families {
            self.remove_partner(*f, id);
        }
        for f in &p.parent_families {
            self.remove_child(*f, id);
        }
        self.people.remove(&id);
        if self.home == Some(id) {
            self.home = None;
        }
        // Other records may point at this person, for example as an associate.
        let pointer = format!("@I{id}@");
        self.for_each_extra(&mut |nodes| nodes.retain(|n| n.value != pointer));
    }

    /// Merges `drop` into `keep`. Blank fields fill in; events, sources and families carry over.
    pub fn merge_people(&mut self, keep: u32, drop: u32) -> Result<(), String> {
        if keep == drop {
            return Err("Those are the same person.".into());
        }
        if self.ancestors(keep).contains(&drop) || self.ancestors(drop).contains(&keep) {
            return Err("One of them is an ancestor of the other, so they can't be merged.".into());
        }
        let Some(gone) = self.people.remove(&drop) else { return Err("That person is gone.".into()) };
        if self.home == Some(drop) {
            self.home = Some(keep);
        }
        for fam in self.families.values_mut() {
            if fam.husband == Some(drop) {
                fam.husband = if fam.wife == Some(keep) { None } else { Some(keep) };
            }
            if fam.wife == Some(drop) {
                fam.wife = if fam.husband == Some(keep) { None } else { Some(keep) };
            }
            if fam.children.contains(&drop) {
                let already = fam.children.contains(&keep);
                fam.children.retain(|c| *c != drop || !already);
                for c in &mut fam.children {
                    if *c == drop {
                        *c = keep;
                    }
                }
            }
        }
        let Some(p) = self.people.get_mut(&keep) else {
            self.people.insert(drop, gone);
            return Err("That person is gone.".into());
        };
        for (mine, theirs) in [
            (&mut p.given, &gone.given),
            (&mut p.surname, &gone.surname),
            (&mut p.suffix, &gone.suffix),
        ] {
            if mine.trim().is_empty() {
                mine.clone_from(theirs);
            }
        }
        if p.sex == Sex::Unknown {
            p.sex = gone.sex;
        }
        if !gone.notes.trim().is_empty() {
            if !p.notes.is_empty() {
                p.notes.push_str("\n\n");
            }
            p.notes.push_str(&gone.notes);
        }
        for e in gone.events {
            if !p.events.contains(&e) {
                p.events.push(e);
            }
        }
        for c in gone.citations {
            if !p.citations.contains(&c) {
                p.citations.push(c);
            }
        }
        for n in gone.name_extra {
            if !p.name_extra.contains(&n) {
                p.name_extra.push(n);
            }
        }
        for n in gone.extra {
            if !p.extra.contains(&n) {
                p.extra.push(n);
            }
        }
        for f in gone.parent_families {
            if !p.parent_families.contains(&f) {
                p.parent_families.push(f);
            }
        }
        for f in gone.own_families {
            if !p.own_families.contains(&f) {
                p.own_families.push(f);
            }
        }
        let (from, to) = (format!("@I{drop}@"), format!("@I{keep}@"));
        self.for_each_extra(&mut |nodes| {
            for n in nodes {
                n.walk_mut(&mut |n| {
                    if n.value == from {
                        n.value.clone_from(&to);
                    }
                });
            }
        });
        Ok(())
    }

    /// People who share a name and don't have birth years that rule it out.
    pub fn possible_duplicates(&self) -> Vec<(u32, u32)> {
        let mut groups: HashMap<(String, String), Vec<&Person>> = HashMap::new();
        for p in self.people.values() {
            let surname = fold(p.surname.trim());
            let given = fold(p.given.split_whitespace().next().unwrap_or(""));
            if !surname.is_empty() && !given.is_empty() {
                groups.entry((surname, given)).or_default().push(p);
            }
        }
        let mut out = vec![];
        let mut ordered: Vec<_> = groups.into_values().filter(|g| g.len() > 1).collect();
        ordered.sort_by_key(|g| g[0].id);
        for group in ordered {
            for (i, a) in group.iter().enumerate() {
                for b in &group[i + 1..] {
                    let close = match (a.birth_year(), b.birth_year()) {
                        (Some(x), Some(y)) => (x - y).abs() <= 2,
                        _ => true,
                    };
                    let related = self.ancestors(a.id).contains(&b.id)
                        || self.ancestors(b.id).contains(&a.id)
                        || a.own_families.iter().any(|f| b.own_families.contains(f))
                        || a.parent_families.iter().any(|f| b.parent_families.contains(f));
                    if close && !related {
                        out.push((a.id, b.id));
                        if out.len() >= 500 {
                            return out;
                        }
                    }
                }
            }
        }
        out
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

    /// Drops links to records that aren't there, which only a damaged file can have.
    pub fn repair_links(&mut self) {
        let (people, families): (HashSet<u32>, HashSet<u32>) =
            (self.people.keys().copied().collect(), self.families.keys().copied().collect());
        for p in self.people.values_mut() {
            p.parent_families.retain(|f| families.contains(f));
            p.own_families.retain(|f| families.contains(f));
        }
        for f in self.families.values_mut() {
            f.husband = f.husband.filter(|p| people.contains(p));
            f.wife = f.wife.filter(|p| people.contains(p));
            f.children.retain(|p| people.contains(p));
        }
    }

    /// A made-up family, shown until the person opens or imports their own.
    pub fn sample() -> Database {
        let mut db = Database::default();
        let ev = |tag: &str, date: &str, place: &str| Event {
            tag: tag.into(),
            date: date.into(),
            place: place.into(),
            ..Default::default()
        };
        let mut add = |given: &str, surname: &str, sex: Sex, events: Vec<Event>, notes: &str| {
            db.add_person(Person {
                given: given.into(),
                surname: surname.into(),
                sex,
                events,
                notes: notes.into(),
                ..Default::default()
            })
        };
        let ada = add(
            "Ada Grace", "Alder", Sex::Female,
            vec![ev("BIRT", "3 MAR 1902", "Ashford, Kent, England"), ev("DEAT", "19 NOV 1984", "Toronto, Ontario, Canada")],
            "A made-up person. Everyone in this sample family is invented.",
        );
        let tom = add(
            "Thomas Edward", "Marsh", Sex::Male,
            vec![ev("BIRT", "14 JUL 1899", "Hythe, Kent, England"), ev("DEAT", "2 FEB 1971", "Toronto, Ontario, Canada")],
            "",
        );
        let walter = add(
            "Walter John", "Alder", Sex::Male,
            vec![ev("BIRT", "21 JAN 1871", "Ashford, Kent, England"), ev("DEAT", "8 AUG 1945", "Ashford, Kent, England")],
            "",
        );
        let rose = add(
            "Rose Ellen", "Finch", Sex::Female,
            vec![ev("BIRT", "11 MAY 1875", "Canterbury, Kent, England"), ev("DEAT", "30 OCT 1950", "Ashford, Kent, England")],
            "",
        );
        let john = add("John", "Alder", Sex::Male, vec![ev("BIRT", "ABT 1840", "Ashford, Kent, England"), ev("DEAT", "1902", "Ashford, Kent, England")], "");
        let mary = add("Mary", "Stone", Sex::Female, vec![ev("BIRT", "1844", "Dover, Kent, England")], "");
        let george = add("George", "Finch", Sex::Male, vec![ev("BIRT", "1848", "Canterbury, Kent, England"), ev("DEAT", "1919", "Canterbury, Kent, England")], "");
        let ellen = add("Ellen", "Hart", Sex::Female, vec![ev("BIRT", "ABT 1851", "Whitstable, Kent, England")], "");
        let june = add("June Rose", "Marsh", Sex::Female, vec![ev("BIRT", "6 JUN 1925", "Toronto, Ontario, Canada")], "");
        let peter = add("Peter Walter", "Marsh", Sex::Male, vec![ev("BIRT", "17 SEP 1928", "Toronto, Ontario, Canada"), ev("DEAT", "4 APR 2003", "Ottawa, Ontario, Canada")], "");
        let helen = add("Helen", "Marsh", Sex::Female, vec![ev("BIRT", "22 DEC 1933", "Toronto, Ontario, Canada")], "");

        let marr = db.add_spouse(ada, tom).expect("sample family");
        for c in [june, peter, helen] {
            db.add_child(ada, Some(marr), c).expect("sample family");
        }
        db.add_parent(ada, walter, true).expect("sample family");
        db.add_parent(ada, rose, false).expect("sample family");
        db.add_parent(walter, john, true).expect("sample family");
        db.add_parent(walter, mary, false).expect("sample family");
        db.add_parent(rose, george, true).expect("sample family");
        db.add_parent(rose, ellen, false).expect("sample family");
        if let Some(f) = db.families.get_mut(&marr) {
            f.events.push(ev("MARR", "9 APR 1924", "Ashford, Kent, England"));
        }
        let parents = db.people[&ada].parent_families[0];
        if let Some(f) = db.families.get_mut(&parents) {
            f.events.push(ev("MARR", "1898", "Canterbury, Kent, England"));
        }
        let sid = db.next_source_id();
        db.sources.insert(sid, Source {
            id: sid,
            title: "Sample parish register".into(),
            author: "Made up for this example".into(),
            ..Default::default()
        });
        if let Some(p) = db.people.get_mut(&ada) {
            p.citations.push(Citation { source: sid, page: "Page 12".into(), ..Default::default() });
        }
        db
    }
}

/// The orders Renumber people can put people in.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RenumberOrder {
    /// The home person, their parents, grandparents and so on, then the rest of the family.
    Family,
    /// The order they are in now, closing the gaps.
    Present,
    /// By surname, then given names; people without a name last.
    Alphabetical,
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

    /// Everyone's ID in the order they would get the new IDs 1, 2, 3...
    pub fn renumber_order(&self, how: RenumberOrder) -> Vec<u32> {
        match how {
            RenumberOrder::Present => self.people.keys().copied().collect(),
            RenumberOrder::Alphabetical => {
                let mut v: Vec<u32> = self.people.keys().copied().collect();
                v.sort_by_cached_key(|id| {
                    let p = &self.people[id];
                    let unnamed = p.given.trim().is_empty() && p.surname.trim().is_empty();
                    (unnamed, search_fold(&p.surname), search_fold(&p.given), *id)
                });
                v
            }
            RenumberOrder::Family => {
                use std::collections::VecDeque;
                let parents = |id: u32| -> Vec<u32> {
                    self.people[&id]
                        .parent_families
                        .iter()
                        .filter_map(|f| self.families.get(f))
                        .flat_map(|f| [f.husband, f.wife])
                        .flatten()
                        .filter(|x| self.people.contains_key(x))
                        .collect()
                };
                let mut out = Vec::with_capacity(self.people.len());
                let mut seen = HashSet::new();
                if let Some(h) = self.home_person() {
                    let mut generation = vec![h];
                    seen.insert(h);
                    out.push(h);
                    while !generation.is_empty() {
                        let mut next = vec![];
                        for id in generation {
                            for p in parents(id) {
                                if seen.insert(p) {
                                    out.push(p);
                                    next.push(p);
                                }
                            }
                        }
                        generation = next;
                    }
                }
                let walk = |queue: &mut VecDeque<u32>, seen: &mut HashSet<u32>, out: &mut Vec<u32>| {
                    while let Some(id) = queue.pop_front() {
                        let mut near = vec![];
                        for f in self.people[&id].own_families.iter().filter_map(|f| self.families.get(f)) {
                            near.extend([f.husband, f.wife].into_iter().flatten());
                            near.extend(f.children.iter().copied());
                        }
                        near.extend(parents(id));
                        for x in near {
                            if self.people.contains_key(&x) && seen.insert(x) {
                                out.push(x);
                                queue.push_back(x);
                            }
                        }
                    }
                };
                let mut queue: VecDeque<u32> = out.iter().copied().collect();
                walk(&mut queue, &mut seen, &mut out);
                for id in self.people.keys().copied().collect::<Vec<_>>() {
                    if seen.insert(id) {
                        out.push(id);
                        queue.push_back(id);
                        walk(&mut queue, &mut seen, &mut out);
                    }
                }
                out
            }
        }
    }

    /// Gives people the IDs 1, 2, 3... in `order` and families 1, 2, 3... by their first member.
    /// Every link follows, including pointers kept in details this program has no field for.
    /// Each person keeps their old ID as "REFN ... TYPE Old ID" (unless they have one already).
    /// Returns old to new person IDs.
    pub fn renumber(&mut self, order: &[u32]) -> HashMap<u32, u32> {
        let mut ids: HashMap<u32, u32> = HashMap::new();
        for id in order.iter().chain(self.people.keys()) {
            if self.people.contains_key(id) && !ids.contains_key(id) {
                let n = ids.len() as u32 + 1;
                ids.insert(*id, n);
            }
        }
        let mut by_new: Vec<(u32, u32)> = ids.iter().map(|(o, n)| (*n, *o)).collect();
        by_new.sort();
        let mut fams: HashMap<u32, u32> = HashMap::new();
        for (_, old) in &by_new {
            let p = &self.people[old];
            for f in p.own_families.iter().chain(&p.parent_families) {
                if self.families.contains_key(f) && !fams.contains_key(f) {
                    let n = fams.len() as u32 + 1;
                    fams.insert(*f, n);
                }
            }
        }
        for f in self.families.keys() {
            if !fams.contains_key(f) {
                let n = fams.len() as u32 + 1;
                fams.insert(*f, n);
            }
        }
        let pid = |id: u32| ids.get(&id).copied().unwrap_or(id);
        let fid = |id: u32| fams.get(&id).copied().unwrap_or(id);
        let mut pointers: HashMap<String, String> = HashMap::new();
        for (o, n) in &ids {
            pointers.insert(format!("@I{o}@"), format!("@I{n}@"));
        }
        for (o, n) in &fams {
            pointers.insert(format!("@F{o}@"), format!("@F{n}@"));
        }
        let people = std::mem::take(&mut self.people);
        for (_, mut p) in people {
            if Database::old_id(&p).is_none() {
                let mut n = Node::new("REFN", &p.id.to_string());
                n.children.push(Node::new("TYPE", "Old ID"));
                p.extra.push(n);
            }
            p.id = pid(p.id);
            p.parent_families.iter_mut().for_each(|f| *f = fid(*f));
            p.own_families.iter_mut().for_each(|f| *f = fid(*f));
            self.people.insert(p.id, p);
        }
        let families = std::mem::take(&mut self.families);
        for (_, mut f) in families {
            f.id = fid(f.id);
            f.husband = f.husband.map(pid);
            f.wife = f.wife.map(pid);
            f.children.iter_mut().for_each(|c| *c = pid(*c));
            self.families.insert(f.id, f);
        }
        let mut swap = |nodes: &mut Vec<Node>| {
            for n in nodes.iter_mut() {
                n.walk_mut(&mut |n| {
                    if let Some(new) = pointers.get(&n.value) {
                        n.value.clone_from(new);
                    }
                });
            }
        };
        // Covers people, families, sources and the other records.
        self.for_each_extra(&mut swap);
        self.home = self.home.map(pid);
        ids
    }
}

#[cfg(test)]
mod renumber_tests {
    use super::*;

    fn shape(db: &Database) -> Vec<String> {
        let n = |id: u32| db.name_of(id);
        let mut v: Vec<String> = db
            .families
            .values()
            .map(|f| {
                let mut kids: Vec<String> = f.children.iter().map(|c| n(*c)).collect();
                kids.sort();
                format!("{:?}+{:?}:{kids:?}", f.husband.map(n), f.wife.map(n))
            })
            .collect();
        for p in db.people.values() {
            for f in p.own_families.iter().chain(&p.parent_families) {
                let fam = &db.families[f];
                assert!(fam.husband == Some(p.id) || fam.wife == Some(p.id) || fam.children.contains(&p.id));
            }
        }
        v.sort();
        v.push(format!("home={:?}", db.home.map(n)));
        v
    }

    #[test]
    fn renumbering_keeps_every_link() {
        let mut db = Database::sample();
        let last = *db.people.keys().last().unwrap();
        db.home = Some(last);
        let first = *db.people.keys().next().unwrap();
        // A detail with a pointer to another person, as an associate from another program.
        let mut asso = Node::new("ASSO", &format!("@I{first}@"));
        asso.children.push(Node::new("RELA", "Friend"));
        db.people.get_mut(&last).unwrap().extra.push(asso);
        let order: Vec<u32> = db.people.keys().rev().copied().collect();
        db.renumber(&order);
        let before = shape(&db);
        for how in [RenumberOrder::Family, RenumberOrder::Alphabetical, RenumberOrder::Present] {
            let order = db.renumber_order(how);
            assert_eq!(order.len(), db.people.len());
            db.renumber(&order);
            assert_eq!(shape(&db), before, "{how:?}");
            assert_eq!(db.people.keys().copied().collect::<Vec<_>>(), (1..=db.people.len() as u32).collect::<Vec<_>>());
            assert_eq!(db.families.keys().copied().collect::<Vec<_>>(), (1..=db.families.len() as u32).collect::<Vec<_>>());
            let h = db.home.unwrap();
            let asso = db.people[&h].extra.iter().find(|n| n.tag == "ASSO").unwrap();
            let target: u32 = asso.value.trim_matches(|c| c == '@' || c == 'I').parse().unwrap();
            assert!(db.people.contains_key(&target));
        }
        db.renumber(&db.renumber_order(RenumberOrder::Family));
        assert_eq!(db.home, Some(1));
        // One Old ID each, from the first renumbering.
        assert!(db.people.values().all(|p| p.extra.iter().filter(|n| n.tag == "REFN").count() == 1));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn person(db: &mut Database, given: &str, sex: Sex) -> u32 {
        db.add_person(Person { given: given.into(), surname: "Test".into(), sex, ..Default::default() })
    }

    #[test]
    fn year_is_found_in_common_date_forms() {
        assert_eq!(date_year("ABT 12 MAR 1850"), Some(1850));
        assert_eq!(date_year("12 MAR 1850/51"), Some(1850));
        assert_eq!(date_year("MAR 987"), Some(987));
        assert_eq!(date_year("12 MAR"), None);
        assert_eq!(date_year(""), None);
    }

    #[test]
    fn links_stay_consistent_and_cycles_are_refused() {
        let mut db = Database::default();
        let a = person(&mut db, "A", Sex::Male);
        let b = person(&mut db, "B", Sex::Male);
        let c = person(&mut db, "C", Sex::Male);
        db.add_parent(b, a, true).unwrap();
        db.add_parent(c, b, true).unwrap();
        assert_eq!(db.father_of(c), Some(b));
        assert!(db.add_parent(a, c, true).is_err(), "someone can't be their own ancestor");
        assert!(db.add_child(c, None, a).is_err());
        assert!(db.add_parent(a, a, true).is_err());
        assert!(db.ancestors(c).contains(&a));
    }

    #[test]
    fn adding_a_sibling_or_second_parent_shares_the_family() {
        let mut db = Database::default();
        let kid = person(&mut db, "Kid", Sex::Female);
        let sib = person(&mut db, "Sib", Sex::Male);
        let mum = person(&mut db, "Mum", Sex::Female);
        let dad = person(&mut db, "Dad", Sex::Male);
        db.add_sibling(kid, sib).unwrap();
        db.add_parent(kid, mum, false).unwrap();
        db.add_parent(kid, dad, true).unwrap();
        assert_eq!(db.families.len(), 1);
        assert_eq!(db.father_of(sib), Some(dad));
        assert_eq!(db.mother_of(sib), Some(mum));
    }

    #[test]
    fn spouse_fills_the_open_family_and_children_carry_over() {
        let mut db = Database::default();
        let mum = person(&mut db, "Mum", Sex::Female);
        let dad = person(&mut db, "Dad", Sex::Male);
        let kid = person(&mut db, "Kid", Sex::Male);
        let f = db.add_child(mum, None, kid).unwrap();
        assert_eq!(db.add_spouse(mum, dad).unwrap(), f);
        assert_eq!(db.families[&f].husband, Some(dad));
        assert_eq!(db.families[&f].wife, Some(mum));
        assert!(db.add_spouse(mum, dad).is_err());
    }

    #[test]
    fn deleting_removes_every_link() {
        let mut db = Database::default();
        let a = person(&mut db, "A", Sex::Male);
        let b = person(&mut db, "B", Sex::Female);
        let c = person(&mut db, "C", Sex::Female);
        let f = db.add_spouse(a, b).unwrap();
        db.add_child(a, Some(f), c).unwrap();
        db.delete_person(a);
        assert!(db.person(a).is_none());
        assert_eq!(db.families[&f].husband, None);
        db.delete_person(c);
        assert!(db.families.is_empty(), "a lone partner with no details isn't a family");
        assert!(db.people[&b].own_families.is_empty());
    }

    #[test]
    fn merge_moves_links_and_fills_blanks() {
        let mut db = Database::default();
        let keep = db.add_person(Person { given: "Ann".into(), surname: "Lee".into(), ..Default::default() });
        let drop = db.add_person(Person {
            given: "Ann".into(),
            surname: "Lee".into(),
            sex: Sex::Female,
            events: vec![Event { tag: "BIRT".into(), date: "1900".into(), ..Default::default() }],
            ..Default::default()
        });
        let husband = person(&mut db, "Bob", Sex::Male);
        let kid = person(&mut db, "Kid", Sex::Male);
        let f = db.add_spouse(drop, husband).unwrap();
        db.add_child(drop, Some(f), kid).unwrap();
        db.merge_people(keep, drop).unwrap();
        assert!(db.person(drop).is_none());
        let p = &db.people[&keep];
        assert_eq!(p.sex, Sex::Female);
        assert_eq!(p.birth_year(), Some(1900));
        assert_eq!(db.families[&f].wife, Some(keep));
        assert_eq!(db.mother_of(kid), Some(keep));
    }

    #[test]
    fn merge_refuses_ancestor_and_finds_duplicates_across_accents() {
        let mut db = Database::default();
        let a = db.add_person(Person { given: "Jose Maria".into(), surname: "Nunez".into(), ..Default::default() });
        let b = db.add_person(Person { given: "José".into(), surname: "Núñez".into(), ..Default::default() });
        assert_eq!(db.possible_duplicates(), vec![(a, b)]);
        db.add_parent(a, b, true).unwrap();
        assert!(db.merge_people(a, b).is_err());
        assert!(db.possible_duplicates().is_empty());
    }

    #[test]
    fn sample_family_is_consistent() {
        let db = Database::sample();
        for f in db.families.values() {
            for c in &f.children {
                assert!(db.people[c].parent_families.contains(&f.id));
            }
            for p in [f.husband, f.wife].into_iter().flatten() {
                assert!(db.people[&p].own_families.contains(&f.id));
            }
        }
    }
}
