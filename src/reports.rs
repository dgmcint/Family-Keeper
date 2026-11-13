//! Printable reports. Each one is plain blocks, shown in the app or written as a web page to print.

use crate::data::*;
use std::collections::HashSet;

pub enum Block {
    Heading(String),
    Line { indent: usize, bold: bool, text: String },
    Gap,
}

pub struct Report {
    pub title: String,
    pub blocks: Vec<Block>,
}

impl Report {
    fn new(title: String) -> Self {
        Report { title, blocks: vec![] }
    }
    fn heading(&mut self, s: impl Into<String>) {
        self.blocks.push(Block::Heading(s.into()));
    }
    fn line(&mut self, indent: usize, s: impl Into<String>) {
        self.blocks.push(Block::Line { indent, bold: false, text: s.into() });
    }
    fn bold(&mut self, indent: usize, s: impl Into<String>) {
        self.blocks.push(Block::Line { indent, bold: true, text: s.into() });
    }
    fn gap(&mut self) {
        self.blocks.push(Block::Gap);
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Pedigree,
    FamilyGroup,
    Ancestors,
    Descendants,
    Individual,
    Surnames,
}

impl Kind {
    pub const ALL: [Kind; 6] = [Kind::Pedigree, Kind::FamilyGroup, Kind::Ancestors, Kind::Descendants, Kind::Individual, Kind::Surnames];
    pub fn label(self) -> &'static str {
        match self {
            Kind::Pedigree => "Pedigree chart",
            Kind::FamilyGroup => "Family group record",
            Kind::Ancestors => "Ancestors, numbered",
            Kind::Descendants => "Descendants",
            Kind::Individual => "Individual summary",
            Kind::Surnames => "All surnames",
        }
    }
    pub fn uses_generations(self) -> bool {
        matches!(self, Kind::Pedigree | Kind::Ancestors | Kind::Descendants)
    }
}

pub fn build(db: &Database, kind: Kind, person: u32, generations: usize) -> Report {
    match kind {
        Kind::Pedigree => pedigree(db, person, generations),
        Kind::FamilyGroup => family_group(db, person),
        Kind::Ancestors => ahnentafel(db, person, generations),
        Kind::Descendants => descendants(db, person, generations),
        Kind::Individual => individual(db, person),
        Kind::Surnames => surnames(db),
    }
}

fn life_line(p: &Person) -> String {
    let mut parts = vec![];
    for (label, tags) in [("b.", ["BIRT", "CHR"]), ("d.", ["DEAT", "BURI"])] {
        if let Some(e) = tags.iter().find_map(|t| p.event(t).filter(|e| !e.summary().is_empty())) {
            parts.push(format!("{label} {}", e.summary()));
        }
    }
    parts.join("; ")
}

fn name_and_life(p: &Person) -> String {
    let life = life_line(p);
    if life.is_empty() { p.name() } else { format!("{} ({life})", p.name()) }
}

fn pedigree(db: &Database, person: u32, generations: usize) -> Report {
    let mut r = Report::new(format!("Pedigree of {}", db.name_of(person)));
    // Ancestor branches can rejoin in a damaged file; the depth limit keeps this finite.
    fn walk(db: &Database, r: &mut Report, id: u32, depth: usize, max: usize) {
        let Some(p) = db.person(id) else { return };
        r.line(depth, name_and_life(p));
        if depth + 1 < max {
            if let Some(f) = db.father_of(id) {
                walk(db, r, f, depth + 1, max);
            }
            if let Some(m) = db.mother_of(id) {
                walk(db, r, m, depth + 1, max);
            }
        }
    }
    walk(db, &mut r, person, 0, generations.clamp(1, 12));
    r
}

fn ahnentafel(db: &Database, person: u32, generations: usize) -> Report {
    let mut r = Report::new(format!("Ancestors of {}", db.name_of(person)));
    let mut level: Vec<(u64, u32)> = vec![(1, person)];
    for g in 0..generations.clamp(1, 30) {
        if level.is_empty() {
            break;
        }
        r.heading(format!("Generation {}", g + 1));
        let mut next = vec![];
        for (n, id) in &level {
            let Some(p) = db.person(*id) else { continue };
            r.line(0, format!("{n}. {}", name_and_life(p)));
            if let Some(f) = db.father_of(*id) {
                next.push((n * 2, f));
            }
            if let Some(m) = db.mother_of(*id) {
                next.push((n * 2 + 1, m));
            }
        }
        level = next;
    }
    r
}

fn add_marriage_lines(db: &Database, r: &mut Report, indent: usize, f: &Family, person: u32) {
    let spouse = f.partner_of(person).and_then(|s| db.person(s));
    let marr = f.event("MARR").map(Event::summary).unwrap_or_default();
    match (spouse, marr.is_empty()) {
        (Some(s), true) => r.line(indent, format!("m. {}", name_and_life(s))),
        (Some(s), false) => r.line(indent, format!("m. {marr} to {}", name_and_life(s))),
        (None, false) => r.line(indent, format!("m. {marr}")),
        (None, true) => {}
    }
}

pub fn descendants(db: &Database, person: u32, generations: usize) -> Report {
    let mut r = Report::new(format!("Descendants of {}", db.name_of(person)));
    fn walk(db: &Database, r: &mut Report, id: u32, depth: usize, max: usize, seen: &mut HashSet<u32>) {
        let Some(p) = db.person(id) else { return };
        if !seen.insert(id) {
            r.line(depth, format!("{} (shown above)", p.name()));
            return;
        }
        r.bold(depth, format!("{}. {}", depth + 1, name_and_life(p)));
        if depth + 1 >= max {
            return;
        }
        for f in p.own_families.iter().filter_map(|f| db.families.get(f)) {
            add_marriage_lines(db, r, depth, f, id);
            for c in &f.children {
                walk(db, r, *c, depth + 1, max, seen);
            }
        }
    }
    walk(db, &mut r, person, 0, generations.clamp(1, 30), &mut HashSet::new());
    r
}

fn event_lines(r: &mut Report, indent: usize, events: &[Event]) {
    for e in events {
        let summary = e.summary();
        r.line(indent, if summary.is_empty() { e.label() } else { format!("{}: {summary}", e.label()) });
        if !e.note.trim().is_empty() {
            r.line(indent + 1, e.note.trim());
        }
    }
}

fn family_group(db: &Database, person: u32) -> Report {
    let mut r = Report::new(format!("Family group record for {}", db.name_of(person)));
    let Some(p) = db.person(person) else { return r };
    let families: Vec<&Family> = p.own_families.iter().filter_map(|f| db.families.get(f)).collect();
    if families.is_empty() {
        r.line(0, format!("{} has no spouse or children recorded.", p.name()));
        return r;
    }
    for f in families {
        for (role, id) in [("Husband", f.husband), ("Wife", f.wife)] {
            r.heading(role);
            match id.and_then(|i| db.person(i)) {
                Some(x) => {
                    r.bold(0, x.name());
                    event_lines(&mut r, 0, &x.events);
                    if let Some(father) = db.father_of(x.id) {
                        r.line(0, format!("Father: {}", db.name_of(father)));
                    }
                    if let Some(mother) = db.mother_of(x.id) {
                        r.line(0, format!("Mother: {}", db.name_of(mother)));
                    }
                }
                None => r.line(0, "Not recorded"),
            }
        }
        if !f.events.is_empty() {
            r.heading("Marriage");
            event_lines(&mut r, 0, &f.events);
        }
        r.heading("Children");
        if f.children.is_empty() {
            r.line(0, "None recorded");
        }
        for (i, c) in f.children.iter().enumerate() {
            let Some(x) = db.person(*c) else { continue };
            r.bold(0, format!("{}. {} ({})", i + 1, x.name(), x.sex.label()));
            event_lines(&mut r, 1, &x.events);
            for own in x.own_families.iter().filter_map(|o| db.families.get(o)) {
                add_marriage_lines(db, &mut r, 1, own, *c);
            }
        }
        r.gap();
    }
    r
}

fn individual(db: &Database, person: u32) -> Report {
    let mut r = Report::new(format!("Summary of {}", db.name_of(person)));
    let Some(p) = db.person(person) else { return r };
    r.line(0, format!("Sex: {}", p.sex.label()));
    r.line(0, format!("ID: {}", p.id));
    if !p.events.is_empty() {
        r.heading("Events");
        event_lines(&mut r, 0, &p.events);
    }
    let parents = [("Father", db.father_of(person)), ("Mother", db.mother_of(person))];
    if parents.iter().any(|(_, x)| x.is_some()) {
        r.heading("Parents");
        for (label, id) in parents {
            if let Some(x) = id.and_then(|i| db.person(i)) {
                r.line(0, format!("{label}: {}", name_and_life(x)));
            }
        }
    }
    for f in p.own_families.iter().filter_map(|f| db.families.get(f)) {
        r.heading("Family");
        add_marriage_lines(db, &mut r, 0, f, person);
        for c in &f.children {
            if let Some(x) = db.person(*c) {
                r.line(1, name_and_life(x));
            }
        }
    }
    if !p.notes.trim().is_empty() {
        r.heading("Notes");
        p.notes.lines().for_each(|l| r.line(0, l));
    }
    let cites: Vec<&Citation> = p.citations.iter().chain(p.events.iter().flat_map(|e| &e.citations)).collect();
    if !cites.is_empty() {
        r.heading("Sources");
        for c in cites {
            let title = db.sources.get(&c.source).map_or("(missing source)", |s| s.title.as_str());
            let page = if c.page.is_empty() { String::new() } else { format!(", {}", c.page) };
            r.line(0, format!("{title}{page}"));
        }
    }
    r
}

fn surnames(db: &Database) -> Report {
    let mut r = Report::new("All surnames".to_string());
    let mut counts: std::collections::BTreeMap<String, usize> = Default::default();
    for p in db.people.values() {
        let s = p.surname.trim();
        if !s.is_empty() {
            *counts.entry(s.to_string()).or_default() += 1;
        }
    }
    if counts.is_empty() {
        r.line(0, "No surnames recorded.");
    }
    for (s, n) in counts {
        r.line(0, format!("{s}: {n}"));
    }
    r
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// A web page, because every computer can print one or save it as a PDF.
pub fn to_html(r: &Report, print_on_open: bool) -> String {
    let mut h = String::from("<!doctype html><html><head><meta charset=\"utf-8\"><title>");
    h.push_str(&escape(&r.title));
    h.push_str("</title><style>body{font:12pt Georgia,'Times New Roman',serif;color:#222;margin:2em auto;max-width:46em;padding:0 1em}h1{font-size:18pt;border-bottom:1px solid #999;padding-bottom:.3em}h2{font-size:13pt;margin:1.2em 0 .3em}p{margin:.15em 0}</style></head><body");
    if print_on_open {
        h.push_str(" onload=\"window.print()\"");
    }
    h.push_str(&format!("><h1>{}</h1>\n", escape(&r.title)));
    for b in &r.blocks {
        match b {
            Block::Heading(s) => h.push_str(&format!("<h2>{}</h2>\n", escape(s))),
            Block::Line { indent, bold, text } => {
                let t = escape(text);
                let t = if *bold { format!("<b>{t}</b>") } else { t };
                h.push_str(&format!("<p style=\"margin-left:{}em\">{t}</p>\n", indent * 2));
            }
            Block::Gap => h.push_str("<hr>\n"),
        }
    }
    h.push_str("</body></html>\n");
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(r: &Report) -> String {
        r.blocks
            .iter()
            .map(|b| match b {
                Block::Heading(s) => s.clone(),
                Block::Line { text, .. } => text.clone(),
                Block::Gap => String::new(),
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn every_report_covers_the_sample_family() {
        let db = Database::sample();
        for kind in Kind::ALL {
            for id in db.people.keys() {
                let r = build(&db, kind, *id, 4);
                assert!(!r.title.is_empty());
                assert!(!r.blocks.is_empty(), "{} is empty", kind.label());
            }
        }
        let ada = 1;
        let t = text(&build(&db, Kind::Ancestors, ada, 4));
        assert!(t.contains("2. Walter John Alder") && t.contains("3. Rose Ellen Finch") && t.contains("4. John Alder"));
        assert!(text(&build(&db, Kind::Descendants, ada, 4)).contains("June Rose Marsh"));
        assert!(text(&build(&db, Kind::FamilyGroup, ada, 4)).contains("Helen Marsh"));
        assert!(text(&build(&db, Kind::Individual, ada, 4)).contains("Sample parish register"));
        assert!(text(&build(&db, Kind::Surnames, ada, 4)).contains("Marsh: 4"));
    }

    #[test]
    fn web_page_escapes_names() {
        let mut db = Database::default();
        let id = db.add_person(Person { given: "<script>".into(), ..Default::default() });
        let html = to_html(&build(&db, Kind::Individual, id, 1), false);
        assert!(!html.contains("<script>"));
        assert!(html.contains("&lt;script&gt;"));
    }
}
