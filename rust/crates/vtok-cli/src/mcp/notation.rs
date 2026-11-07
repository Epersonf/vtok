//! The notation rules of each language, shown once per server process together with the first code in that language.
//! They used to be part of the server instructions, which every session of every project carries on every turn whether or
//! not a source file is ever read (about 1,000 tokens of the 1,400 the instructions cost); here they cost tokens only
//! when the notation is actually used.
use std::collections::HashSet;
use std::sync::{Mutex, OnceLock};
use vtok_api::languages;

static SHOWN: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();

/// The rules of `lang` the first time this process shows code in it; an empty string every time after, and for a name that
/// is no language.
pub(super) fn first_use(lang: &str) -> String {
    let fresh = SHOWN.get_or_init(Default::default).lock().map_or(false, |mut shown| shown.insert(lang.to_string()));
    if !fresh { return String::new(); }
    languages().into_iter().find(|l| l.name() == lang).map_or(String::new(), |l| format!("# notation of {lang}\n{}\n", l.grammar_prompt().trim_end()))
}
