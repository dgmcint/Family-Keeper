//! The family file: people, families and sources.

mod db;
mod events;
mod links;
mod merge;
mod records;
mod renumber;
mod sample;

pub use db::*;
pub use events::*;
pub use records::*;
pub use renumber::*;


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
