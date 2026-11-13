//! Renumbering record numbers in a chosen order.

use super::*;
use std::collections::{HashMap, HashSet};

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
