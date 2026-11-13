//! The kinds of events and their names.


/// Person events and facts, in the order they're offered.
pub const PERSON_EVENTS: &[&str] = &[
    "BIRT", "CHR", "DEAT", "BURI", "CREM", "ADOP", "BAPM", "BARM", "BASM", "BLES", "CHRA", "CONF",
    "FCOM", "ORDN", "NATU", "EMIG", "IMMI", "CENS", "PROB", "WILL", "GRAD", "RETI", "RESI", "OCCU",
    "EDUC", "RELI", "TITL", "NATI", "CAST", "DSCR", "IDNO", "PROP", "SSN", "EVEN", "FACT", "BAPL",
    "CONL", "ENDL", "SLGC",
];

/// Family events, in the order they're offered.
pub const FAMILY_EVENTS: &[&str] = &[
    "MARR", "ENGA", "MARB", "MARC", "MARL", "MARS", "DIV", "DIVF", "ANUL", "CENS", "RESI", "EVEN",
    "SLGS",
];

pub fn event_label(tag: &str) -> &str {
    match tag {
        "BIRT" => "Birth",
        "CHR" => "Christening",
        "DEAT" => "Death",
        "BURI" => "Burial",
        "CREM" => "Cremation",
        "ADOP" => "Adoption",
        "BAPM" => "Baptism",
        "BARM" => "Bar mitzvah",
        "BASM" => "Bat mitzvah",
        "BLES" => "Blessing",
        "CHRA" => "Adult christening",
        "CONF" => "Confirmation",
        "FCOM" => "First communion",
        "ORDN" => "Ordination",
        "NATU" => "Naturalization",
        "EMIG" => "Emigration",
        "IMMI" => "Immigration",
        "CENS" => "Census",
        "PROB" => "Probate",
        "WILL" => "Will",
        "GRAD" => "Graduation",
        "RETI" => "Retirement",
        "RESI" => "Residence",
        "OCCU" => "Occupation",
        "EDUC" => "Education",
        "RELI" => "Religion",
        "TITL" => "Title",
        "NATI" => "Nationality",
        "CAST" => "Caste",
        "DSCR" => "Description",
        "IDNO" => "ID number",
        "PROP" => "Property",
        "SSN" => "Social Security number",
        "EVEN" => "Other event",
        "FACT" => "Other fact",
        "BAPL" => "Baptism (LDS)",
        "CONL" => "Confirmation (LDS)",
        "ENDL" => "Endowment (LDS)",
        "SLGC" => "Sealing to parents (LDS)",
        "SLGS" => "Sealing to spouse (LDS)",
        "MARR" => "Marriage",
        "ENGA" => "Engagement",
        "MARB" => "Marriage banns",
        "MARC" => "Marriage contract",
        "MARL" => "Marriage license",
        "MARS" => "Marriage settlement",
        "DIV" => "Divorce",
        "DIVF" => "Divorce filed",
        "ANUL" => "Annulment",
        other => other,
    }
}

pub fn is_lds_event(tag: &str) -> bool {
    matches!(tag, "BAPL" | "CONL" | "ENDL" | "SLGC" | "SLGS")
}
