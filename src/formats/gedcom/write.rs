//! Writing a family file out as GEDCOM.

use super::*;
use crate::data::*;

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
