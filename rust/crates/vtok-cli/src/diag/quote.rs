//! Quoted names in diagnostic messages: `Cannot find name 'foo'.` -> template `Cannot find name '…'.` plus the arg `foo`.
//! Errors that differ only in that name can then share one message line.

const OPEN_AFTER: &str = " ([{,:=<>";
const CLOSE_BEFORE: &str = " .,;:)]}?!'";
/// More quoted spans than this and the message is prose, not a template with one name.
const MAX_SPANS: usize = 3;

/// Byte ranges (quote to quote, inclusive) of the quoted spans in `msg`. An apostrophe inside a word is not a quote.
fn spans(msg: &str) -> Vec<(usize, usize)> {
    let mut out = vec![];
    let chars: Vec<(usize, char)> = msg.char_indices().collect();
    let mut i = 0;
    while i < chars.len() {
        let (at, c) = chars[i];
        let opens = matches!(c, '\'' | '`' | '"') && (i == 0 || OPEN_AFTER.contains(chars[i - 1].1));
        if opens {
            let close = (i + 2..chars.len()).find(|&j| chars[j].1 == c && chars.get(j + 1).map_or(true, |n| CLOSE_BEFORE.contains(n.1)));
            if let Some(j) = close.filter(|&j| chars[j].0 - at <= 80) {
                out.push((at, chars[j].0 + c.len_utf8()));
                i = j + 1;
                continue;
            }
        }
        i += 1;
    }
    out
}

/// One `(template, arg)` per quoted span: the message with that span masked, and the text of the span. Two errors can share
/// a template line when they have a masked version in common (`Parameter '…' implicitly has an 'any' type.`).
pub(super) fn templates(msg: &str) -> Vec<(String, String)> {
    let sp = spans(msg);
    if sp.len() > MAX_SPANS { return vec![]; }
    sp.iter().map(|&(a, b)| {
        let q = &msg[a..a + 1];
        (format!("{}{q}…{q}{}", &msg[..a], &msg[b..]), msg[a + 1..b - 1].to_string())
    }).collect()
}
