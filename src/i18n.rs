//! The menus and windows in other languages.
//!
//! Each language is a gettext `.po` file in the `lang` folder, built into the program, so
//! translators can work on them with ordinary tools such as Poedit. `lang/template.pot` lists
//! every phrase. A regional language (Latin American Spanish, Portugal Portuguese) only lists
//! the phrases that differ and takes the rest from its main language.
//!
//! Family data and printed reports are not translated.
use std::collections::HashMap;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicUsize, Ordering};

struct Language {
    /// The language's name in its own words, as the Language menu shows it.
    name: &'static str,
    code: &'static str,
    /// Older codes saved by earlier versions that mean this language.
    also: &'static [&'static str],
    /// The language that fills in any phrase this one doesn't list.
    base: Option<&'static str>,
    po: &'static str,
}

const ALL: &[Language] = &[
    Language { name: "English", code: "en", also: &[], base: None, po: "" },
    Language { name: "Deutsch", code: "de", also: &[], base: None, po: include_str!("../lang/de.po") },
    Language { name: "Español (España)", code: "es", also: &[], base: None, po: include_str!("../lang/es.po") },
    Language { name: "Español (Latinoamérica)", code: "es-419", also: &[], base: Some("es"), po: include_str!("../lang/es-419.po") },
    Language { name: "Français", code: "fr", also: &[], base: None, po: include_str!("../lang/fr.po") },
    Language { name: "Norsk", code: "nb", also: &["no"], base: None, po: include_str!("../lang/nb.po") },
    Language { name: "Português (Brasil)", code: "pt-BR", also: &["pt"], base: None, po: include_str!("../lang/pt-BR.po") },
    Language { name: "Português (Portugal)", code: "pt-PT", also: &[], base: Some("pt-BR"), po: include_str!("../lang/pt-PT.po") },
    Language { name: "Русский", code: "ru", also: &[], base: None, po: include_str!("../lang/ru.po") },
    Language { name: "Svenska", code: "sv", also: &[], base: None, po: include_str!("../lang/sv.po") },
    Language { name: "日本語", code: "ja", also: &[], base: None, po: include_str!("../lang/ja.po") },
    Language { name: "한국어", code: "ko", also: &[], base: None, po: include_str!("../lang/ko.po") },
    Language { name: "中文 (繁體)", code: "zh-Hant", also: &["zh"], base: None, po: include_str!("../lang/zh-Hant.po") },
];

/// Each language's name in its own words, in menu order. English comes first.
pub const LANGUAGES: [&str; ALL.len()] = {
    let mut names = [""; ALL.len()];
    let mut i = 0;
    while i < ALL.len() {
        names[i] = ALL[i].name;
        i += 1;
    }
    names
};

static CURRENT: AtomicUsize = AtomicUsize::new(0);

pub fn current() -> usize {
    CURRENT.load(Ordering::Relaxed)
}

pub fn set(index: usize) {
    CURRENT.store(index.min(ALL.len() - 1), Ordering::Relaxed);
}

pub fn code() -> &'static str {
    ALL[current()].code
}

/// Chooses a language by its code, such as `de` or `pt-BR`. Unknown codes change nothing.
pub fn set_code(code: &str) {
    let code = code.trim();
    if let Some(i) = ALL.iter().position(|l| l.code.eq_ignore_ascii_case(code) || l.also.iter().any(|a| a.eq_ignore_ascii_case(code))) {
        set(i);
    }
}

/// One phrase's translation: one form, or one per plural form for phrases with a count.
type Catalog = HashMap<String, Vec<String>>;

fn catalogs() -> &'static [Catalog] {
    static ONCE: OnceLock<Vec<Catalog>> = OnceLock::new();
    ONCE.get_or_init(|| ALL.iter().map(|l| parse_po(l.po).into_iter().filter(|e| e.forms.iter().all(|f| !f.is_empty())).map(|e| (e.id, e.forms)).collect()).collect())
}

/// The forms for this phrase in a language, or in its main language.
fn lookup(language: usize, id: &str) -> Option<&'static [String]> {
    let mut index = language;
    loop {
        if let Some(forms) = catalogs()[index].get(id) {
            return Some(forms);
        }
        let base = ALL[index].base?;
        index = ALL.iter().position(|l| l.code == base)?;
    }
}

fn translate<'a>(language: usize, text: &'a str) -> &'a str {
    if language == 0 {
        return text;
    }
    match lookup(language, text) {
        Some(forms) => &forms[0],
        None => text,
    }
}

/// This text in the chosen language. Anything without a translation stays as it is.
pub fn tr<'a>(text: &'a str) -> &'a str {
    translate(current(), text)
}

/// This text in the chosen language, with each `{name}` filled in, for example
/// `trf("Exported to {file}.", &[("file", &path.display())])`.
pub fn trf(text: &str, values: &[(&str, &dyn std::fmt::Display)]) -> String {
    fill(tr(text), values)
}

/// A phrase with a count in it, such as "{n} people", in the right form for the number.
/// `one` is the English for exactly one and `many` for any other number. `{n}` becomes the
/// number and each other `{name}` its value.
pub fn tr_n(one: &str, many: &str, n: usize, values: &[(&str, &dyn std::fmt::Display)]) -> String {
    plural_in(current(), one, many, n, values)
}

fn plural_in(language: usize, one: &str, many: &str, n: usize, values: &[(&str, &dyn std::fmt::Display)]) -> String {
    let english = if n == 1 { one } else { many };
    let text = match (language, lookup(language, one)) {
        (0, _) | (_, None) => english,
        (_, Some(forms)) => forms.get(plural_form(ALL[language].code, n as u64)).or(forms.last()).map_or(english, |s| s.as_str()),
    };
    fill(&text.replace("{n}", &n.to_string()), values)
}

fn fill(text: &str, values: &[(&str, &dyn std::fmt::Display)]) -> String {
    let mut out = text.to_string();
    for (name, value) in values {
        out = out.replace(&format!("{{{name}}}"), &value.to_string());
    }
    out
}

/// Which plural form a number takes, numbered as in each language's `Plural-Forms` header.
fn plural_form(code: &str, n: u64) -> usize {
    match code {
        "ja" | "ko" | "zh-Hant" => 0,
        "fr" | "pt-BR" => usize::from(n > 1),
        "ru" => {
            let (ten, hundred) = (n % 10, n % 100);
            if ten == 1 && hundred != 11 {
                0
            } else if (2..=4).contains(&ten) && !(12..=14).contains(&hundred) {
                1
            } else {
                2
            }
        }
        _ => usize::from(n != 1),
    }
}

struct Entry {
    id: String,
    #[cfg_attr(not(test), allow(dead_code))]
    plural: Option<String>,
    forms: Vec<String>,
}

/// Reads the parts of a .po file this program uses: msgid, msgid_plural and msgstr entries,
/// with strings split over several lines. Comments and the header are skipped.
fn parse_po(text: &str) -> Vec<Entry> {
    let mut entries = Vec::new();
    let mut id: Option<String> = None;
    let mut plural: Option<String> = None;
    let mut forms: Vec<String> = Vec::new();
    // Which string a continuation line adds to: 0 msgid, 1 msgid_plural, 2.. msgstr forms.
    let mut target = 0usize;
    let mut finish = |id: &mut Option<String>, plural: &mut Option<String>, forms: &mut Vec<String>| {
        if let Some(i) = id.take() {
            if !i.is_empty() {
                entries.push(Entry { id: i, plural: plural.take(), forms: std::mem::take(forms) });
            }
        }
        *plural = None;
        forms.clear();
    };
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(rest) = line.strip_prefix("msgid_plural ") {
            plural = Some(unquote(rest));
            target = 1;
        } else if let Some(rest) = line.strip_prefix("msgid ") {
            finish(&mut id, &mut plural, &mut forms);
            id = Some(unquote(rest));
            target = 0;
        } else if let Some(rest) = line.strip_prefix("msgstr[") {
            let (n, rest) = rest.split_once(']').unwrap_or(("0", rest));
            let n: usize = n.parse().unwrap_or(0);
            if forms.len() <= n {
                forms.resize(n + 1, String::new());
            }
            forms[n] = unquote(rest.trim());
            target = 2 + n;
        } else if let Some(rest) = line.strip_prefix("msgstr ") {
            forms = vec![unquote(rest)];
            target = 2;
        } else if line.starts_with('"') {
            let more = unquote(line);
            match target {
                0 => id.get_or_insert_with(String::new).push_str(&more),
                1 => plural.get_or_insert_with(String::new).push_str(&more),
                n => {
                    if let Some(f) = forms.get_mut(n - 2) {
                        f.push_str(&more);
                    }
                }
            }
        }
    }
    finish(&mut id, &mut plural, &mut forms);
    entries
}

/// The text inside a quoted .po string, with its escapes turned back into characters.
fn unquote(s: &str) -> String {
    let s = s.trim();
    let s = s.strip_prefix('"').unwrap_or(s);
    let s = s.strip_suffix('"').unwrap_or(s);
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some(other) => out.push(other),
            None => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    const TEMPLATE: &str = include_str!("../lang/template.pot");

    fn index(code: &str) -> usize {
        ALL.iter().position(|l| l.code == code).unwrap()
    }

    fn placeholders(s: &str) -> BTreeSet<String> {
        let mut out = BTreeSet::new();
        let mut rest = s;
        while let Some(start) = rest.find('{') {
            let Some(end) = rest[start..].find('}') else { break };
            out.insert(rest[start..start + end + 1].to_string());
            rest = &rest[start + end + 1..];
        }
        out
    }

    fn forms_for(code: &str) -> usize {
        match code {
            "ja" | "ko" | "zh-Hant" => 1,
            "ru" => 3,
            _ => 2,
        }
    }

    #[test]
    fn every_language_has_the_main_menu_and_unknown_text_stays() {
        let words = [("en", "File"), ("de", "Datei"), ("es", "Archivo"), ("fr", "Fichier"), ("nb", "Fil"), ("pt-BR", "Arquivo"), ("pt-PT", "Ficheiro"), ("ru", "Файл"), ("sv", "Fil"), ("ja", "ファイル"), ("ko", "파일"), ("zh-Hant", "檔案")];
        for (code, word) in words {
            assert_eq!(translate(index(code), "File"), word, "{code}");
            assert_eq!(translate(index(code), "a name nobody translates"), "a name nobody translates");
        }
    }

    #[test]
    fn regional_languages_fall_back_to_their_main_language() {
        assert_eq!(translate(index("pt-PT"), "Help"), translate(index("pt-BR"), "Help"));
        assert_eq!(translate(index("es-419"), "Help"), "Ayuda");
    }

    #[test]
    fn codes_choose_languages_and_old_codes_still_work() {
        for code in ["zh", "no", "pt"] {
            let found = ALL.iter().position(|l| l.also.contains(&code));
            assert!(found.is_some(), "{code}");
        }
        assert_eq!(LANGUAGES[0], "English");
        assert_eq!(LANGUAGES.len(), ALL.len());
    }

    #[test]
    fn plural_forms_follow_each_language() {
        assert_eq!([1, 2, 5, 11, 21, 22, 25].map(|n| plural_form("ru", n)), [0, 1, 2, 2, 0, 1, 2]);
        assert_eq!([0, 1, 2].map(|n| plural_form("fr", n)), [0, 0, 1]);
        assert_eq!([0, 1, 2].map(|n| plural_form("de", n)), [1, 0, 1]);
        assert_eq!(plural_form("ja", 7), 0);
        assert_eq!(plural_in(0, "{n} person", "{n} people", 1, &[]), "1 person");
        assert_eq!(plural_in(0, "{n} person", "{n} people", 3, &[]), "3 people");
        assert_eq!(fill("{a} and {b}", &[("a", &1), ("b", &"two")]), "1 and two");
    }

    #[test]
    fn full_languages_translate_every_phrase_the_same_way() {
        let template = parse_po(TEMPLATE);
        let ids: BTreeSet<&str> = template.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids.len(), template.len(), "a phrase is listed twice in template.pot");
        for lang in ALL.iter().skip(1) {
            let entries = parse_po(lang.po);
            let mut seen = BTreeSet::new();
            for e in &entries {
                assert!(seen.insert(e.id.as_str()), "{}: listed twice: {:?}", lang.code, e.id);
                let Some(t) = template.iter().find(|t| t.id == e.id) else { panic!("{}: not in template.pot: {:?}", lang.code, e.id) };
                assert_eq!(e.plural, t.plural, "{}: plural differs from template.pot for {:?}", lang.code, e.id);
                let want = if e.plural.is_some() { forms_for(lang.code) } else { 1 };
                assert_eq!(e.forms.len(), want, "{}: wrong number of forms for {:?}", lang.code, e.id);
                let english: BTreeSet<String> = placeholders(&e.id).into_iter().chain(e.plural.iter().flat_map(|p| placeholders(p))).collect();
                for f in &e.forms {
                    assert!(!f.is_empty(), "{}: empty translation for {:?}", lang.code, e.id);
                    // A count's form for exactly one may write the word instead of {n}.
                    let mut have = placeholders(f);
                    if e.plural.is_some() {
                        have.insert("{n}".into());
                    }
                    assert_eq!(have, english, "{}: placeholders differ in {:?}", lang.code, e.id);
                }
            }
            if lang.base.is_none() {
                let missing: Vec<_> = ids.iter().filter(|id| !seen.contains(*id)).collect();
                assert!(missing.is_empty(), "{}: {} phrases not translated, first {:?}", lang.code, missing.len(), missing.first());
            }
        }
    }

    #[test]
    fn every_phrase_in_the_code_is_in_the_template() {
        let template = parse_po(TEMPLATE);
        let mut missing = Vec::new();
        let mut stack = vec![std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/src"))];
        while let Some(dir) = stack.pop() {
            for entry in std::fs::read_dir(&dir).unwrap().flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                    continue;
                }
                if path.extension().is_none_or(|e| e != "rs") || path.ends_with("i18n.rs") {
                    continue;
                }
                let text = std::fs::read_to_string(&path).unwrap();
                for call in ["tr(\"", "trf(\"", "tr_n(\""] {
                    for (at, _) in text.match_indices(call) {
                        if text[..at].ends_with(|c: char| c.is_alphanumeric() || c == '_') {
                            continue;
                        }
                        let Some((one, rest)) = literal(&text[at + call.len() - 1..]) else { continue };
                        if !one.chars().any(char::is_alphabetic) {
                            continue;
                        }
                        let plural = if call == "tr_n(\"" { literal(rest.trim_start().strip_prefix(',').unwrap_or(rest).trim_start()).map(|(p, _)| p) } else { None };
                        match template.iter().find(|e| e.id == one) {
                            Some(e) if e.plural == plural => {}
                            _ => missing.push(format!("{}: {:?}", path.display(), one)),
                        }
                    }
                }
            }
        }
        assert!(missing.is_empty(), "phrases missing from lang/template.pot:\n{}", missing.join("\n"));
    }

    /// The Rust string literal at the start of `s`, with simple escapes resolved, and what follows it.
    fn literal(s: &str) -> Option<(String, &str)> {
        let body = s.strip_prefix('"')?;
        let mut out = String::new();
        let mut chars = body.char_indices();
        while let Some((i, c)) = chars.next() {
            match c {
                '"' => return Some((out, &body[i + 1..])),
                '\\' => match chars.next()?.1 {
                    'n' => out.push('\n'),
                    't' => out.push('\t'),
                    '\n' => {
                        // A line continuation: skip the next line's leading spaces.
                        while chars.clone().next().is_some_and(|(_, c)| c == ' ') {
                            chars.next();
                        }
                    }
                    other => out.push(other),
                },
                _ => out.push(c),
            }
        }
        None
    }
}
