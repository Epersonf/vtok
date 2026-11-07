//! Token-level patch: only the changed tokens are rewritten, the original spacing and line breaks stay.
use crate::language::Language;

fn is_word(c: char) -> bool { c.is_alphanumeric() || c == '_' || c == '$' }

/// Token-level patch: turn `old` into `new` by keeping the original text (spacing, line breaks) of every
/// token that did not change and writing only the changed tokens. None = not applicable.
pub(super) fn token_patch(lang: &dyn Language, old: &str, new: &str, member: bool) -> Option<String> {
    let ot = lang.tokens(old, member)?;
    let nt = lang.tokens(new, member)?;
    if ot.is_empty() || nt.is_empty() { return None; }
    let os: Vec<&str> = ot.iter().map(|&(a, b)| &old[a..b]).collect();
    let ns: Vec<&str> = nt.iter().map(|&(a, b)| &new[a..b]).collect();
    let pre = os.iter().zip(&ns).take_while(|(a, b)| a == b).count();
    let suf = os[pre..].iter().rev().zip(ns[pre..].iter().rev()).take_while(|(a, b)| a == b).count();
    let (om, nm) = (&os[pre..os.len() - suf], &ns[pre..ns.len() - suf]);
    if om.len().saturating_mul(nm.len()) > 4_000_000 { return None; }
    // LCS over the middle
    let (n, m) = (om.len(), nm.len());
    let w = m + 1;
    let mut t = vec![0u32; (n + 1) * w];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            t[i * w + j] = if om[i] == nm[j] { t[(i + 1) * w + j + 1] + 1 } else { t[(i + 1) * w + j].max(t[i * w + j + 1]) };
        }
    }
    // ops: (old index, new index); deletes before inserts inside a change region
    let mut ops: Vec<(Option<usize>, Option<usize>)> = (0..pre).map(|i| (Some(i), Some(i))).collect();
    let (mut i, mut j) = (0, 0);
    while i < n || j < m {
        if i < n && j < m && om[i] == nm[j] { ops.push((Some(pre + i), Some(pre + j))); i += 1; j += 1; }
        else if i < n && (j >= m || t[(i + 1) * w + j] >= t[i * w + j + 1]) { ops.push((Some(pre + i), None)); i += 1; }
        else { ops.push((None, Some(pre + j))); j += 1; }
    }
    for k in 0..suf { ops.push((Some(os.len() - suf + k), Some(ns.len() - suf + k))); }
    let gap = |text: &str, toks: &[(usize, usize)], i: usize| -> String {
        let s = if i == 0 { 0 } else { toks[i - 1].1 };
        text[s..toks[i].0].to_string()
    };
    let mut out = String::new(); // the first gap (taken from 0) already carries the leading whitespace
    let mut dels: std::collections::VecDeque<usize> = Default::default();
    for (o, nn) in ops {
        let (g, tok) = match (o, nn) {
            (Some(i), Some(_)) => { dels.clear(); (gap(old, &ot, i), os[i]) }
            (Some(i), None) => { dels.push_back(i); continue; }
            (None, Some(k)) => {
                let g = match dels.pop_front() {
                    Some(d) => { let g = gap(old, &ot, d); if g.chars().all(char::is_whitespace) { g } else { gap(new, &nt, k) } }
                    None => gap(new, &nt, k),
                };
                (g, ns[k])
            }
            _ => continue,
        };
        out.push_str(&g);
        if g.is_empty() && out.chars().last().map_or(false, is_word) && tok.chars().next().map_or(false, is_word) { out.push(' '); }
        out.push_str(tok);
    }
    out.push_str(&old[ot.last()?.1..]);
    Some(out)
}
