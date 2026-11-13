//! The made-up sample family shown on first start.

use super::*;

impl Database {
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
