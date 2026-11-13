//! Adding and removing the links between people and families.

use super::*;
use std::collections::HashSet;

impl Database {
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

    pub(crate) fn for_each_extra(&mut self, f: &mut impl FnMut(&mut Vec<Node>)) {
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
}
