//! Finding possible duplicates and merging two people.

use super::*;
use std::collections::HashMap;

impl Database {
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
}
