//! Reading and writing GEDCOM, the file format family history programs share.
//! Anything without a field of its own is kept and written back out.

mod read;
mod text;
mod write;

pub use read::*;
use text::*;
pub use write::*;

use crate::data::*;

pub struct Imported {
    pub db: Database,
    pub warnings: Vec<String>,
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
