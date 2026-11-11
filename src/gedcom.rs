//! Reading and writing GEDCOM, the file format family history programs share.
//! Anything without a field of its own is kept and written back out.

use crate::model::*;
use std::collections::{HashMap, HashSet};
use unicode_normalization::UnicodeNormalization;

pub struct Imported {
    pub db: Database,
    pub warnings: Vec<String>,
}

// ---------- Characters ----------

fn ansel_spacing(b: u8) -> Option<char> {
    Some(match b {
        0xA1 => 'Ł', 0xA2 => 'Ø', 0xA3 => 'Đ', 0xA4 => 'Þ', 0xA5 => 'Æ', 0xA6 => 'Œ', 0xA7 => 'ʹ',
        0xA8 => '·', 0xA9 => '♭', 0xAA => '®', 0xAB => '±', 0xAC => 'Ơ', 0xAD => 'Ư', 0xAE => 'ʼ',
        0xB0 => 'ʻ', 0xB1 => 'ł', 0xB2 => 'ø', 0xB3 => 'đ', 0xB4 => 'þ', 0xB5 => 'æ', 0xB6 => 'œ',
        0xB7 => 'ʺ', 0xB8 => 'ı', 0xB9 => '£', 0xBA => 'ð', 0xBC => 'ơ', 0xBD => 'ư', 0xC0 => '°',
        0xC1 => 'ℓ', 0xC2 => '℗', 0xC3 => '©', 0xC4 => '♯', 0xC5 => '¿', 0xC6 => '¡', 0xCF => 'ß',
        _ => return None,
    })
}

fn ansel_combining(b: u8) -> Option<char> {
    Some(match b {
        0xE0 => '\u{0309}', 0xE1 => '\u{0300}', 0xE2 => '\u{0301}', 0xE3 => '\u{0302}',
        0xE4 => '\u{0303}', 0xE5 => '\u{0304}', 0xE6 => '\u{0306}', 0xE7 => '\u{0307}',
        0xE8 => '\u{0308}', 0xE9 => '\u{030C}', 0xEA => '\u{030A}', 0xEB => '\u{FE20}',
        0xEC => '\u{FE21}', 0xED => '\u{0315}', 0xEE => '\u{030B}', 0xEF => '\u{0310}',
        0xF0 => '\u{0327}', 0xF1 => '\u{0328}', 0xF2 => '\u{0323}', 0xF3 => '\u{0324}',
        0xF4 => '\u{0325}', 0xF5 => '\u{0333}', 0xF6 => '\u{0332}', 0xF7 => '\u{0326}',
        0xF8 => '\u{031C}', 0xF9 => '\u{032E}', 0xFA => '\u{FE22}', 0xFB => '\u{FE23}',
        0xFE => '\u{0313}',
        _ => return None,
    })
}

/// ANSEL puts accents before their letter; Unicode puts them after.
fn decode_ansel(bytes: &[u8]) -> (String, usize) {
    let mut out = String::with_capacity(bytes.len());
    let mut pending: Vec<char> = vec![];
    let mut unknown = 0;
    for &b in bytes {
        if let Some(mark) = ansel_combining(b) {
            pending.push(mark);
            continue;
        }
        let c = if b < 0x80 {
            b as char
        } else if let Some(c) = ansel_spacing(b) {
            c
        } else {
            unknown += 1;
            '\u{FFFD}'
        };
        out.push(c);
        // A line break can't carry an accent; don't move it onto the next line's first letter.
        if c == '\n' || c == '\r' {
            pending.clear();
        } else {
            out.extend(pending.drain(..).rev());
        }
    }
    (out.nfc().collect(), unknown)
}

fn declared_charset(bytes: &[u8]) -> Option<String> {
    // The header is plain ASCII, so it can be read before the rest is decoded.
    let head = &bytes[..bytes.len().min(8192)];
    let text: String = head.iter().map(|b| *b as char).collect();
    text.split(['\r', '\n']).find_map(|line| {
        let mut parts = line.split_whitespace();
        (parts.next() == Some("1") && parts.next() == Some("CHAR"))
            .then(|| parts.next().unwrap_or("").to_ascii_uppercase())
    })
}

fn decode(bytes: &[u8], warnings: &mut Vec<String>) -> String {
    if let Some(rest) = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        return String::from_utf8_lossy(rest).into_owned();
    }
    let utf16 = if bytes.starts_with(&[0xFF, 0xFE]) || (bytes.len() > 1 && bytes[0] == b'0' && bytes[1] == 0) {
        Some(encoding_rs::UTF_16LE)
    } else if bytes.starts_with(&[0xFE, 0xFF]) || (bytes.len() > 1 && bytes[0] == 0 && bytes[1] == b'0') {
        Some(encoding_rs::UTF_16BE)
    } else {
        None
    };
    if let Some(enc) = utf16 {
        let (text, _, bad) = enc.decode(bytes);
        if bad {
            warnings.push("Some characters couldn't be read and show as \u{FFFD}.".into());
        }
        return text.trim_start_matches('\u{FEFF}').to_string();
    }
    let charset = declared_charset(bytes).unwrap_or_default();
    match charset.as_str() {
        "ANSEL" => {
            let (text, unknown) = decode_ansel(bytes);
            if unknown > 0 {
                warnings.push(format!("{unknown} characters couldn't be read and show as \u{FFFD}."));
            }
            text
        }
        "MACINTOSH" | "MACROMAN" => encoding_rs::MACINTOSH.decode(bytes).0.into_owned(),
        _ => match std::str::from_utf8(bytes) {
            Ok(s) => s.to_string(),
            Err(_) => {
                if !matches!(charset.as_str(), "ANSI" | "ASCII" | "WINDOWS-1252" | "CP1252" | "ISO-8859-1" | "LATIN1") {
                    warnings.push(format!(
                        "The file's character set ({}) wasn't recognised, so it was read as Windows Latin. Check accented letters.",
                        if charset.is_empty() { "not stated" } else { &charset }
                    ));
                }
                encoding_rs::WINDOWS_1252.decode(bytes).0.into_owned()
            }
        },
    }
}

// ---------- Lines to a tree ----------

fn is_pointer(value: &str) -> bool {
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

// ---------- Tree to records ----------

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

// ---------- Records to a file ----------

/// Longest piece of a value on one line, in bytes.
const MAX_VALUE: usize = 200;

fn write_value(out: &mut String, level: usize, head: &str, value: &str) {
    let escaped;
    let value = if is_pointer(value) || value.starts_with("@#") || !value.contains('@') {
        value
    } else {
        escaped = value.replace('@', "@@");
        &escaped
    };
    let normalized = value.replace("\r\n", "\n").replace('\r', "\n");
    let cont = format!("{} CONT", level + 1);
    for (i, line) in normalized.split('\n').enumerate() {
        write_chunks(out, level, if i == 0 { head } else { &cont }, line);
    }
}

fn write_chunks(out: &mut String, level: usize, first_prefix: &str, line: &str) {
    let conc = format!("{} CONC", level + 1);
    let mut prefix = first_prefix;
    let mut rest = line;
    loop {
        if rest.len() <= MAX_VALUE {
            out.push_str(prefix);
            if !rest.is_empty() {
                out.push(' ');
                out.push_str(rest);
            }
            out.push_str("\r\n");
            return;
        }
        // Breaking beside a space would lose it in programs that trim lines.
        let mut cut = MAX_VALUE;
        while !rest.is_char_boundary(cut) {
            cut -= 1;
        }
        let mut better = cut;
        while better > 1
            && (!rest.is_char_boundary(better)
                || rest[..better].ends_with(' ')
                || rest[better..].starts_with(' '))
        {
            better -= 1;
        }
        if better > 1 {
            cut = better;
        }
        out.push_str(prefix);
        out.push(' ');
        out.push_str(&rest[..cut]);
        out.push_str("\r\n");
        rest = &rest[cut..];
        prefix = &conc;
    }
}

fn line(out: &mut String, level: usize, tag: &str, value: &str) {
    write_value(out, level, &format!("{level} {tag}"), value);
}

fn line_if(out: &mut String, level: usize, tag: &str, value: &str) {
    if !value.is_empty() {
        line(out, level, tag, value);
    }
}

fn write_node(out: &mut String, level: usize, n: &Node) {
    let head = match &n.xref {
        Some(x) => format!("{level} @{x}@ {}", n.tag),
        None => format!("{level} {}", n.tag),
    };
    write_value(out, level, &head, &n.value);
    for c in &n.children {
        write_node(out, level + 1, c);
    }
}

fn write_citation(out: &mut String, level: usize, c: &Citation) {
    line(out, level, "SOUR", &format!("@S{}@", c.source));
    line_if(out, level + 1, "PAGE", &c.page);
    if !c.text.is_empty() {
        line(out, level + 1, "DATA", "");
        line(out, level + 2, "TEXT", &c.text);
    }
    for n in &c.extra {
        write_node(out, level + 1, n);
    }
}

fn write_event(out: &mut String, e: &Event) {
    line(out, 1, &e.tag, &e.value);
    line_if(out, 2, "TYPE", &e.kind);
    if !e.date.is_empty() || !e.date_extra.is_empty() {
        line(out, 2, "DATE", &e.date);
        e.date_extra.iter().for_each(|n| write_node(out, 3, n));
    }
    line_if(out, 2, "TEMP", &e.temple);
    if !e.place.is_empty() || !e.place_extra.is_empty() {
        line(out, 2, "PLAC", &e.place);
        e.place_extra.iter().for_each(|n| write_node(out, 3, n));
    }
    line_if(out, 2, "NOTE", &e.note);
    e.citations.iter().for_each(|c| write_citation(out, 2, c));
    e.extra.iter().for_each(|n| write_node(out, 2, n));
}

pub fn export(db: &Database) -> String {
    let mut out = String::new();
    let submitter = db.other_records.iter().find(|r| r.tag == "SUBM").and_then(|r| r.xref.clone());
    out.push_str("0 HEAD\r\n1 SOUR FAMILY-FILE\r\n2 NAME Family Keeper\r\n2 VERS ");
    out.push_str(env!("CARGO_PKG_VERSION"));
    out.push_str("\r\n1 GEDC\r\n2 VERS 5.5.1\r\n2 FORM LINEAGE-LINKED\r\n1 CHAR UTF-8\r\n");
    let home = db.home.filter(|h| db.people.contains_key(h));
    if let Some(h) = home {
        // Read back by this program, PAF Classic and Gilded Roots, so the home person stays.
        out.push_str(&format!("1 _HOME @I{h}@\r\n"));
    }
    out.push_str(&format!("1 SUBM @{}@\r\n", submitter.as_deref().unwrap_or("SUBM1")));
    if submitter.is_none() {
        out.push_str("0 @SUBM1@ SUBM\r\n1 NAME Family Keeper user\r\n");
    }

    // The home person first, so programs that take the first person as home agree.
    let first = home.and_then(|h| db.people.get(&h));
    for p in first.into_iter().chain(db.people.values().filter(|p| Some(p.id) != home)) {
        out.push_str(&format!("0 @I{}@ INDI\r\n", p.id));
        let surname = p.surname.trim();
        let mut name = format!("{} /{surname}/", p.given.trim());
        if !p.suffix.trim().is_empty() {
            name.push(' ');
            name.push_str(p.suffix.trim());
        }
        line(&mut out, 1, "NAME", name.trim());
        line_if(&mut out, 2, "GIVN", p.given.trim());
        line_if(&mut out, 2, "SURN", surname);
        line_if(&mut out, 2, "NSFX", p.suffix.trim());
        p.name_extra.iter().for_each(|n| write_node(&mut out, 2, n));
        match p.sex {
            Sex::Male => line(&mut out, 1, "SEX", "M"),
            Sex::Female => line(&mut out, 1, "SEX", "F"),
            Sex::Unknown => {}
        }
        p.events.iter().for_each(|e| write_event(&mut out, e));
        for f in &p.parent_families {
            let pointer = format!("@F{f}@");
            line(&mut out, 1, "FAMC", &pointer);
            for detail in p.extra.iter().filter(|n| n.tag == "_FAMC_DETAIL" && n.value == pointer) {
                detail.children.iter().for_each(|n| write_node(&mut out, 2, n));
            }
        }
        for f in &p.own_families {
            line(&mut out, 1, "FAMS", &format!("@F{f}@"));
        }
        line_if(&mut out, 1, "NOTE", &p.notes);
        p.citations.iter().for_each(|c| write_citation(&mut out, 1, c));
        for n in p.extra.iter().filter(|n| n.tag != "_FAMC_DETAIL") {
            write_node(&mut out, 1, n);
        }
    }

    for f in db.families.values() {
        out.push_str(&format!("0 @F{}@ FAM\r\n", f.id));
        if let Some(h) = f.husband {
            line(&mut out, 1, "HUSB", &format!("@I{h}@"));
        }
        if let Some(w) = f.wife {
            line(&mut out, 1, "WIFE", &format!("@I{w}@"));
        }
        for c in &f.children {
            line(&mut out, 1, "CHIL", &format!("@I{c}@"));
        }
        f.events.iter().for_each(|e| write_event(&mut out, e));
        line_if(&mut out, 1, "NOTE", &f.notes);
        f.citations.iter().for_each(|c| write_citation(&mut out, 1, c));
        f.extra.iter().for_each(|n| write_node(&mut out, 1, n));
    }

    for s in db.sources.values() {
        out.push_str(&format!("0 @S{}@ SOUR\r\n", s.id));
        line_if(&mut out, 1, "TITL", &s.title);
        line_if(&mut out, 1, "AUTH", &s.author);
        line_if(&mut out, 1, "PUBL", &s.publication);
        line_if(&mut out, 1, "TEXT", &s.text);
        line_if(&mut out, 1, "NOTE", &s.notes);
        s.extra.iter().for_each(|n| write_node(&mut out, 1, n));
    }

    db.other_records.iter().for_each(|r| write_node(&mut out, 0, r));
    out.push_str("0 TRLR\r\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "0 HEAD\n1 SOUR OLD\n1 GEDC\n2 VERS 5.5\n1 CHAR UTF-8\n\
0 @I5@ INDI\n1 NAME Thomas Edward /Marsh/ Jr\n2 _CUSTOM keep me\n1 SEX M\n1 BIRT\n2 DATE ABT 1899\n2 PLAC Hythe, Kent\n3 MAP\n4 LATI N51\n2 SOUR @S9@\n3 PAGE 12\n3 QUAY 2\n\
1 OCCU Farmer\n1 _UID ABC123\n1 FAMS @F2@\n1 NOTE First line\n2 CONT Second \n2 CONC line\n1 ASSO @I6@\n2 RELA Friend\n\
0 @I6@ INDI\n1 NAME Ada /Alder/\n1 SEX F\n1 ENDL\n2 DATE 1920\n2 TEMP LOGAN\n1 FAMS @F2@\n1 NOTE @N1@\n\
0 @I7@ INDI\n1 NAME June //\n1 FAMC @F2@\n2 PEDI adopted\n\
0 @F2@ FAM\n1 HUSB @I5@\n1 WIFE @I6@\n1 CHIL @I7@\n1 MARR\n2 DATE 1924\n1 SLGS\n2 TEMP SLAKE\n\
0 @S9@ SOUR\n1 TITL Parish register\n1 AUTH A. Clerk\n1 REPO @R1@\n\
0 @R1@ REPO\n1 NAME County office\n0 @N1@ NOTE Shared words\n0 TRLR\n";

    fn imported() -> Database {
        import(SAMPLE.as_bytes()).unwrap().db
    }

    #[test]
    fn reads_people_families_and_links() {
        let db = imported();
        assert_eq!(db.people.len(), 3);
        let t = &db.people[&5];
        assert_eq!((t.given.as_str(), t.surname.as_str(), t.suffix.as_str()), ("Thomas Edward", "Marsh", "Jr"));
        assert_eq!(t.sex, Sex::Male);
        assert_eq!(t.date_of("BIRT"), "ABT 1899");
        assert_eq!(t.notes, "First line\nSecond line");
        assert_eq!(t.event("OCCU").unwrap().value, "Farmer");
        assert_eq!(db.father_of(7), Some(5));
        assert_eq!(db.mother_of(7), Some(6));
        assert_eq!(db.people[&6].notes, "Shared words");
        assert_eq!(db.people[&6].event("ENDL").unwrap().temple, "LOGAN");
        assert_eq!(db.sources[&9].title, "Parish register");
        assert_eq!(t.events[0].citations[0].page, "12");
    }

    #[test]
    fn nothing_is_lost_exporting_and_reading_back() {
        let first = imported();
        let text = export(&first);
        let second = import(text.as_bytes()).unwrap();
        assert!(second.warnings.is_empty(), "{:?}", second.warnings);
        // The format requires a submitter record, so the first export adds one; it then stays put.
        let mut back = second.db.clone();
        back.other_records.retain(|r| r.tag != "SUBM");
        assert_eq!(first, back);
        assert_eq!(second.db, import(export(&second.db).as_bytes()).unwrap().db);
        for must_keep in ["_CUSTOM keep me", "1 _UID ABC123", "4 LATI N51", "3 QUAY 2", "2 PEDI adopted", "1 ASSO @I6@", "0 @R1@ REPO", "1 REPO @R1@"] {
            assert!(text.contains(must_keep), "lost {must_keep}\n{text}");
        }
        assert!(!text.contains("_FAMC_DETAIL"));
    }

    #[test]
    fn long_text_and_at_signs_survive() {
        let mut db = Database::default();
        let note = format!("{}\n\nmail me: a@b.org ends here  {}", "wörd ".repeat(200), "é".repeat(500));
        db.add_person(Person { given: "Long".into(), notes: note.clone(), ..Default::default() });
        let text = export(&db);
        assert!(text.lines().all(|l| l.len() <= 255));
        assert!(text.contains("a@@b.org"));
        assert_eq!(import(text.as_bytes()).unwrap().db.people[&1].notes, note);
    }

    #[test]
    fn date_escapes_are_left_alone() {
        let mut db = Database::default();
        let mut e = Event::new("BIRT");
        e.date = "@#DJULIAN@ 1 JAN 1700".into();
        db.add_person(Person { given: "Old".into(), events: vec![e], ..Default::default() });
        let back = import(export(&db).as_bytes()).unwrap().db;
        assert_eq!(back.people[&1].date_of("BIRT"), "@#DJULIAN@ 1 JAN 1700");
    }

    #[test]
    fn reads_ansel_accents_and_special_letters() {
        let mut bytes = b"0 HEAD\r\n1 CHAR ANSEL\r\n0 @I1@ INDI\r\n1 NAME Jos".to_vec();
        bytes.extend([0xE2, b'e']); // é
        bytes.extend(b" /M");
        bytes.extend([0xE8, b'u']); // ü
        bytes.extend(b"ller ");
        bytes.extend([0xB2]); // ø
        bytes.extend(b"/\r\n0 TRLR\r\n");
        let db = import(&bytes).unwrap().db;
        assert_eq!(db.people[&1].given, "José");
        assert_eq!(db.people[&1].surname, "Müller ø");
    }

    #[test]
    fn reads_utf16_and_windows_latin() {
        let text = "0 HEAD\r\n1 CHAR UNICODE\r\n0 @I1@ INDI\r\n1 NAME Zoë /Núñez/\r\n0 TRLR\r\n";
        let mut bytes = vec![0xFF, 0xFE];
        text.encode_utf16().for_each(|u| bytes.extend(u.to_le_bytes()));
        assert_eq!(import(&bytes).unwrap().db.people[&1].given, "Zoë");

        let mut latin = b"0 HEAD\r\n1 CHAR ANSI\r\n0 @I1@ INDI\r\n1 NAME Zo".to_vec();
        latin.push(0xEB);
        latin.extend(b" //\r\n0 TRLR\r\n");
        let got = import(&latin).unwrap();
        assert_eq!(got.db.people[&1].given, "Zoë");
        assert!(got.warnings.is_empty());
    }

    #[test]
    fn refuses_other_files_and_survives_damaged_ones() {
        assert!(import(b"hello, this is not a family file").is_err());
        assert!(import(b"").is_err());
        assert!(import(&[0xFF; 64]).is_err());
        let messy = "0 HEAD\n1 CHAR UTF-8\n3 ORPHAN x\n0 @I1@ INDI\n1 NAME A /B/\n1 FAMS @F9@\n1 SOUR @S404@\n0 @I1@ INDI\n1 NAME Dup /Dup/\n0 @F1@ FAM\n1 CHIL @I404@\n5 DEEP y\nstray words\n0 @I2@ INDI\n1 NAME Self /Loop/\n1 FAMC @F3@\n1 FAMS @F3@\n0 @F3@ FAM\n1 HUSB @I2@\n1 CHIL @I2@\n0 TRLR\n";
        let got = import(messy.as_bytes()).unwrap();
        assert_eq!(got.db.people[&1].given, "A");
        assert!(got.warnings.len() >= 3, "{:?}", got.warnings);
        assert!(got.db.sources.values().any(|s| s.title.contains("S404")));
        // A file claiming someone is their own parent mustn't hang anything.
        assert!(got.db.ancestors(2).contains(&2));
        assert!(!crate::reports::descendants(&got.db, 2, 50).blocks.is_empty());
        import(export(&got.db).as_bytes()).unwrap();
    }

    #[test]
    fn home_person_comes_from_the_file_and_survives_export() {
        let text = "0 HEAD\n1 CHAR UTF-8\n0 @I900000001@ INDI\n1 NAME Ann /Lee/\n1 FAMC @F777777@\n0 @I12345678@ INDI\n1 NAME Joe /Lee/\n1 FAMS @F777777@\n0 @F777777@ FAM\n1 HUSB @I12345678@\n1 CHIL @I900000001@\n0 TRLR\n";
        let db = import(text.as_bytes()).unwrap().db;
        // Website IDs become 1, 2... in file order, and the first person is home.
        assert_eq!(db.people.keys().copied().collect::<Vec<_>>(), vec![1, 2]);
        assert_eq!(db.home, Some(1));
        assert_eq!(db.people[&1].given, "Ann");
        let mut db = db;
        db.home = Some(2);
        let out = export(&db);
        assert!(out.contains("1 _HOME @I2@"), "{out}");
        assert!(out.find("0 @I2@ INDI").unwrap() < out.find("0 @I1@ INDI").unwrap());
        let again = import(out.as_bytes()).unwrap().db;
        assert_eq!(again.name_of(again.home.unwrap()), "Joe Lee");
        // Without a header mark, the first person in the file is home.
        let plain = out.replace("1 _HOME @I2@\r\n", "");
        let third = import(plain.as_bytes()).unwrap().db;
        assert_eq!(third.name_of(third.home.unwrap()), "Joe Lee");
    }

    #[test]
    fn different_id_styles_get_numbers_without_clashing() {
        let text = "0 HEAD\n1 CHAR UTF-8\n0 @P1@ INDI\n1 NAME A //\n0 @X1@ INDI\n1 NAME B //\n0 @NOPE@ INDI\n1 NAME C //\n1 ASSO @X1@\n0 TRLR\n";
        let db = import(text.as_bytes()).unwrap().db;
        assert_eq!(db.people.len(), 3);
        let b = db.people.values().find(|p| p.given == "B").unwrap().id;
        let c = db.people.values().find(|p| p.given == "C").unwrap();
        assert_eq!(c.extra[0].value, format!("@I{b}@"));
    }
}
