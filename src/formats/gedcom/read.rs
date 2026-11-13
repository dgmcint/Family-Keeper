//! Reading a GEDCOM file into a family file.

use super::*;
use crate::data::*;
use std::collections::{HashMap, HashSet};

pub(crate) fn is_pointer(value: &str) -> bool {
    value.len() > 2
        && value.starts_with('@')
        && value.ends_with('@')
        && !value.starts_with("@#")
        && !value[1..value.len() - 1].contains('@')
}

struct Line<'a> {
    level: usize,
    xref: Option<&'a str>,
    tag: &'a str,
    value: &'a str,
}

fn split_word(s: &str) -> Option<(&str, &str)> {
    let s = s.trim_start();
    if s.is_empty() {
        return None;
    }
    Some(match s.find([' ', '\t']) {
        Some(i) => (&s[..i], &s[i + 1..]),
        None => (s, ""),
    })
}

fn parse_line(line: &str) -> Option<Line<'_>> {
    let (level, rest) = split_word(line)?;
    let level: usize = level.parse().ok().filter(|l| *l < 100)?;
    let (mut word, mut rest) = split_word(rest)?;
    let mut xref = None;
    if word.len() > 2 && word.starts_with('@') && word.ends_with('@') {
        xref = Some(&word[1..word.len() - 1]);
        (word, rest) = split_word(rest)?;
    }
    if !word.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return None;
    }
    Some(Line { level, xref, tag: word, value: rest })
}

fn close_to(stack: &mut Vec<Node>, roots: &mut Vec<Node>, keep: usize) {
    while stack.len() > keep {
        let node = stack.pop().expect("length checked");
        match stack.last_mut() {
            Some(parent) => parent.children.push(node),
            None => roots.push(node),
        }
    }
}

fn parse_tree(text: &str, warnings: &mut Vec<String>) -> Vec<Node> {
    let mut roots = vec![];
    let mut stack: Vec<Node> = vec![];
    let mut bad = 0;
    for raw in text.split(['\r', '\n']) {
        if raw.trim().is_empty() {
            continue;
        }
        let Some(line) = parse_line(raw) else {
            // Old programs sometimes broke long text onto a bare line. Keep it as more text.
            if let Some(top) = stack.last_mut() {
                top.value.push('\n');
                top.value.push_str(raw.trim());
            }
            bad += 1;
            continue;
        };
        if line.tag == "CONC" || line.tag == "CONT" {
            if let Some(top) = stack.last_mut() {
                if line.tag == "CONT" {
                    top.value.push('\n');
                }
                top.value.push_str(line.value);
                continue;
            }
        }
        // A line that jumps a level down attaches to the deepest open line.
        let keep = line.level.min(stack.len());
        if line.level > 0 && keep == 0 {
            bad += 1;
            continue;
        }
        close_to(&mut stack, &mut roots, keep);
        stack.push(Node {
            tag: line.tag.to_ascii_uppercase(),
            xref: line.xref.map(str::to_string),
            value: line.value.to_string(),
            children: vec![],
        });
    }
    close_to(&mut stack, &mut roots, 0);
    if bad > 0 {
        warnings.push(format!("{bad} lines weren't in GEDCOM form; they were kept as text where possible."));
    }
    for r in &mut roots {
        r.walk_mut(&mut |n| {
            if !is_pointer(&n.value) {
                n.value = n.value.replace("@@", "@");
            }
        });
    }
    roots
}

struct Importer {
    warnings: Vec<String>,
    /// old "X" in "@X@" → new number, by record kind
    people: HashMap<String, u32>,
    families: HashMap<String, u32>,
    sources: HashMap<String, u32>,
    shared_notes: HashMap<String, String>,
    new_sources: Vec<Source>,
    next_source: u32,
}

fn number_in(xref: &str) -> Option<u32> {
    let digits: String = xref.chars().filter(char::is_ascii_digit).collect();
    digits.parse().ok().filter(|n| *n > 0 && *n < u32::MAX / 2)
}

/// Gives each record a number. The numbers in the file's IDs are kept when they look like record
/// numbers (no bigger than ten times the number of records), as in files from this program and
/// PAF; the long IDs of Ancestry and other websites are replaced by 1, 2, 3... in file order.
fn assign(roots: &[Node], tag: &str) -> HashMap<String, u32> {
    let mut used = HashSet::new();
    let mut map = HashMap::new();
    let records: Vec<&String> = roots.iter().filter(|r| r.tag == tag).filter_map(|r| r.xref.as_ref()).collect();
    let limit = (records.len() as u64).saturating_mul(10).max(100);
    if records.iter().any(|x| number_in(x).is_none_or(|n| n as u64 > limit)) {
        for x in &records {
            if !map.contains_key(*x) {
                let n = map.len() as u32 + 1;
                map.insert((*x).clone(), n);
            }
        }
        return map;
    }
    for x in &records {
        if map.contains_key(*x) {
            continue;
        }
        if let Some(n) = number_in(x) {
            if used.insert(n) {
                map.insert((*x).clone(), n);
            }
        }
    }
    let mut next = used.iter().max().copied().unwrap_or(0) + 1;
    for x in &records {
        if !map.contains_key(*x) {
            map.insert((*x).clone(), next);
            next += 1;
        }
    }
    map
}

fn pointer_target(value: &str) -> Option<&str> {
    is_pointer(value).then(|| &value[1..value.len() - 1])
}

impl Importer {
    fn note_text(&self, n: &Node) -> String {
        match pointer_target(&n.value) {
            Some(x) => self.shared_notes.get(x).cloned().unwrap_or_default(),
            None => n.value.clone(),
        }
    }

    fn citation(&mut self, n: &Node) -> Citation {
        let mut c = Citation::default();
        match pointer_target(&n.value) {
            Some(x) => match self.sources.get(x) {
                Some(id) => c.source = *id,
                None => {
                    c.source = self.new_source(format!("Missing source (@{x}@)"), String::new());
                    self.warnings.push(format!("A citation pointed to source @{x}@, which isn't in the file."));
                }
            },
            None => {
                // Some programs write the source's words right on the citation.
                let title: String = n.value.lines().next().unwrap_or("").chars().take(120).collect();
                c.source = self.new_source(title, n.value.clone());
            }
        }
        for k in &n.children {
            match k.tag.as_str() {
                "PAGE" if c.page.is_empty() => c.page.clone_from(&k.value),
                "TEXT" if c.text.is_empty() => c.text.clone_from(&k.value),
                "DATA" if k.children.len() == 1 && k.children[0].tag == "TEXT" && c.text.is_empty() => {
                    c.text.clone_from(&k.children[0].value);
                }
                _ => c.extra.push(k.clone()),
            }
        }
        c
    }

    fn new_source(&mut self, title: String, text: String) -> u32 {
        let id = self.next_source;
        self.next_source += 1;
        self.new_sources.push(Source { id, title, text, ..Default::default() });
        id
    }

    fn event(&mut self, n: &Node) -> Event {
        let mut e = Event { tag: n.tag.clone(), value: n.value.clone(), ..Default::default() };
        for k in &n.children {
            match k.tag.as_str() {
                "TYPE" if e.kind.is_empty() => e.kind.clone_from(&k.value),
                "DATE" if e.date.is_empty() => {
                    e.date.clone_from(&k.value);
                    e.date_extra.clone_from(&k.children);
                }
                "PLAC" if e.place.is_empty() => {
                    e.place.clone_from(&k.value);
                    e.place_extra.clone_from(&k.children);
                }
                "TEMP" if e.temple.is_empty() => e.temple.clone_from(&k.value),
                "NOTE" => {
                    let t = self.note_text(k);
                    append_note(&mut e.note, &t);
                }
                "SOUR" => {
                    let c = self.citation(k);
                    e.citations.push(c);
                }
                _ => e.extra.push(k.clone()),
            }
        }
        e
    }

    fn person(&mut self, n: &Node, id: u32) -> Person {
        let mut p = Person { id, ..Default::default() };
        let mut has_name = false;
        for k in &n.children {
            match k.tag.as_str() {
                "NAME" if !has_name => {
                    has_name = true;
                    let (given, surname, suffix) = split_name(&k.value);
                    (p.given, p.surname, p.suffix) = (given, surname, suffix);
                    for sub in &k.children {
                        match sub.tag.as_str() {
                            "GIVN" if p.given.is_empty() => p.given.clone_from(&sub.value),
                            "SURN" if p.surname.is_empty() => p.surname.clone_from(&sub.value),
                            "NSFX" if p.suffix.is_empty() => p.suffix.clone_from(&sub.value),
                            "GIVN" | "SURN" | "NSFX" => {}
                            _ => p.name_extra.push(sub.clone()),
                        }
                    }
                }
                "SEX" => {
                    p.sex = match k.value.trim().chars().next().map(|c| c.to_ascii_uppercase()) {
                        Some('M') => Sex::Male,
                        Some('F') => Sex::Female,
                        _ => Sex::Unknown,
                    }
                }
                "FAMC" | "FAMS" => {} // read in the second pass over links
                "NOTE" => {
                    let t = self.note_text(k);
                    append_note(&mut p.notes, &t);
                }
                "SOUR" => {
                    let c = self.citation(k);
                    p.citations.push(c);
                }
                tag if PERSON_EVENTS.contains(&tag) => {
                    let e = self.event(k);
                    p.events.push(e);
                }
                _ => p.extra.push(k.clone()),
            }
        }
        p
    }

    fn family(&mut self, n: &Node, id: u32) -> Family {
        let mut f = Family { id, ..Default::default() };
        for k in &n.children {
            match k.tag.as_str() {
                "HUSB" | "WIFE" | "CHIL" => {} // read in the second pass over links
                "NOTE" => {
                    let t = self.note_text(k);
                    append_note(&mut f.notes, &t);
                }
                "SOUR" => {
                    let c = self.citation(k);
                    f.citations.push(c);
                }
                tag if FAMILY_EVENTS.contains(&tag) => {
                    let e = self.event(k);
                    f.events.push(e);
                }
                _ => f.extra.push(k.clone()),
            }
        }
        f
    }
}

fn append_note(to: &mut String, text: &str) {
    if text.is_empty() {
        return;
    }
    if !to.is_empty() {
        to.push_str("\n\n");
    }
    to.push_str(text);
}

/// "Thomas Edward /Marsh/ Jr" → given, surname, suffix.
pub fn split_name(value: &str) -> (String, String, String) {
    let tidy = |s: &str| s.split_whitespace().collect::<Vec<_>>().join(" ");
    match value.find('/') {
        None => (tidy(value), String::new(), String::new()),
        Some(a) => {
            let after = &value[a + 1..];
            let (surname, suffix) = match after.find('/') {
                Some(b) => (&after[..b], &after[b + 1..]),
                None => (after, ""),
            };
            (tidy(&value[..a]), tidy(surname), tidy(suffix).trim_start_matches(',').trim().to_string())
        }
    }
}

fn link_partner(db: &mut Database, family: u32, person: u32, role: &str) {
    let fam = db.families.get_mut(&family).expect("family exists");
    if fam.husband == Some(person) || fam.wife == Some(person) {
        return;
    }
    let as_husband = if role == "HUSB" { fam.husband.is_none() } else { fam.wife.is_some() };
    if as_husband {
        fam.husband = Some(person);
    } else {
        fam.wife = Some(person);
    }
    let p = db.people.get_mut(&person).expect("person exists");
    if !p.own_families.contains(&family) {
        p.own_families.push(family);
    }
}

fn link_child(db: &mut Database, family: u32, person: u32) {
    let fam = db.families.get_mut(&family).expect("family exists");
    if !fam.children.contains(&person) {
        fam.children.push(person);
    }
    let p = db.people.get_mut(&person).expect("person exists");
    if !p.parent_families.contains(&family) {
        p.parent_families.push(family);
    }
}

pub fn import(bytes: &[u8]) -> Result<Imported, String> {
    let mut warnings = vec![];
    let text = decode(bytes, &mut warnings);
    let mut roots = parse_tree(&text, &mut warnings);
    if roots.first().map(|r| r.tag.as_str()) != Some("HEAD") {
        return Err("This doesn't look like a GEDCOM file: it doesn't start with a GEDCOM header.".into());
    }

    let people = assign(&roots, "INDI");
    let families = assign(&roots, "FAM");
    let sources = assign(&roots, "SOUR");
    let shared_notes: HashMap<String, String> = roots
        .iter()
        .filter(|r| r.tag == "NOTE")
        .filter_map(|r| Some((r.xref.clone()?, r.value.clone())))
        .collect();

    // Pointers in records kept as they are (associates, aliases) follow the new numbers.
    let mut rewrite: HashMap<String, String> = HashMap::new();
    for (x, n) in &people {
        rewrite.insert(format!("@{x}@"), format!("@I{n}@"));
    }
    for (x, n) in &families {
        rewrite.insert(format!("@{x}@"), format!("@F{n}@"));
    }
    for r in &mut roots {
        for k in &mut r.children {
            k.walk_mut(&mut |n| {
                if !matches!(n.tag.as_str(), "HUSB" | "WIFE" | "CHIL" | "FAMC" | "FAMS") {
                    if let Some(new) = rewrite.get(&n.value) {
                        n.value.clone_from(new);
                    }
                }
            });
        }
    }

    let next_source = sources.values().max().copied().unwrap_or(0) + 1;
    let mut im = Importer {
        warnings,
        people: people.clone(),
        families: families.clone(),
        sources,
        shared_notes,
        new_sources: vec![],
        next_source,
    };
    let mut db = Database::default();
    let mut seen: HashSet<(String, String)> = HashSet::new();
    let mut dup = 0;

    for r in &roots {
        let Some(x) = r.xref.clone() else {
            if !matches!(r.tag.as_str(), "HEAD" | "TRLR") {
                db.other_records.push(r.clone());
            }
            continue;
        };
        let known = matches!(r.tag.as_str(), "INDI" | "FAM" | "SOUR" | "NOTE");
        if known && !seen.insert((r.tag.clone(), x.clone())) {
            dup += 1;
            continue;
        }
        match r.tag.as_str() {
            "INDI" => {
                let id = im.people[&x];
                let p = im.person(r, id);
                db.people.insert(id, p);
            }
            "FAM" => {
                let id = im.families[&x];
                let f = im.family(r, id);
                db.families.insert(id, f);
            }
            "SOUR" => {
                let id = im.sources[&x];
                let mut s = Source { id, ..Default::default() };
                let mut abbr = String::new();
                for k in &r.children {
                    match k.tag.as_str() {
                        "TITL" if s.title.is_empty() => s.title.clone_from(&k.value),
                        "AUTH" if s.author.is_empty() => s.author.clone_from(&k.value),
                        "PUBL" if s.publication.is_empty() => s.publication.clone_from(&k.value),
                        "TEXT" if s.text.is_empty() => s.text.clone_from(&k.value),
                        "NOTE" => {
                            let t = im.note_text(k);
                            append_note(&mut s.notes, &t);
                        }
                        "ABBR" => {
                            abbr.clone_from(&k.value);
                            s.extra.push(k.clone());
                        }
                        _ => s.extra.push(k.clone()),
                    }
                }
                if s.title.is_empty() {
                    s.title = abbr;
                }
                db.sources.insert(id, s);
            }
            "NOTE" => {} // folded into whatever uses it
            _ => db.other_records.push(r.clone()),
        }
    }
    for s in std::mem::take(&mut im.new_sources) {
        db.sources.insert(s.id, s);
    }

    // Second pass: links. Either side of a link counts, so one-sided files still connect.
    let mut dangling = 0;
    let target = |map: &HashMap<String, u32>, n: &Node| pointer_target(&n.value).and_then(|x| map.get(x).copied());
    let mut linked: HashSet<(String, String)> = HashSet::new();
    for r in roots.iter().filter(|r| r.tag == "FAM") {
        let Some(x) = r.xref.as_ref() else { continue };
        if !linked.insert(("FAM".into(), x.clone())) {
            continue;
        }
        let fam = families[x];
        for k in r.children.iter().filter(|k| matches!(k.tag.as_str(), "HUSB" | "WIFE" | "CHIL")) {
            match target(&people, k) {
                Some(p) if k.tag == "CHIL" => link_child(&mut db, fam, p),
                Some(p) => link_partner(&mut db, fam, p, &k.tag),
                None => dangling += 1,
            }
        }
    }
    for r in roots.iter().filter(|r| r.tag == "INDI") {
        let Some(x) = r.xref.as_ref() else { continue };
        if !linked.insert(("INDI".into(), x.clone())) {
            continue;
        }
        let id = people[x];
        for k in r.children.iter().filter(|k| k.tag == "FAMC" || k.tag == "FAMS") {
            match target(&families, k) {
                Some(f) if k.tag == "FAMC" => {
                    link_child(&mut db, f, id);
                    // Keep details on a parent link (adopted, foster...) that have no field here.
                    if !k.children.is_empty() {
                        let mut kept = k.clone();
                        kept.tag = "_FAMC_DETAIL".into();
                        kept.value = format!("@F{f}@");
                        db.people.get_mut(&id).expect("person exists").extra.push(kept);
                    }
                }
                Some(f) => {
                    let male = db.people[&id].sex == Sex::Male;
                    link_partner(&mut db, f, id, if male { "HUSB" } else { "WIFE" });
                }
                None => dangling += 1,
            }
        }
    }
    let mut warnings = im.warnings;
    if dangling > 0 {
        warnings.push(format!("{dangling} family links pointed to records that aren't in the file and were left out."));
    }
    if dup > 0 {
        warnings.push(format!("{dup} records repeated an ID already used; only the first of each was kept."));
    }
    // The home person: the one the file names in its header (as this program, PAF Classic and
    // Gilded Roots write it), otherwise the first person in the file, as PAF and Ancestry do.
    let named = roots
        .first()
        .and_then(|h| h.child("_HOME"))
        .and_then(|k| pointer_target(&k.value))
        .and_then(|x| people.get(x).copied());
    let first = roots.iter().find(|r| r.tag == "INDI").and_then(|r| r.xref.as_ref()).and_then(|x| people.get(x).copied());
    db.home = named.or(first);
    Ok(Imported { db, warnings })
}
