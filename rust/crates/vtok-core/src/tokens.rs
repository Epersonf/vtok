//! Token counting with the o200k_base tokenizer (a proxy for Claude's tokenizer).
use std::sync::OnceLock;
use tiktoken_rs::CoreBPE;

fn bpe() -> &'static CoreBPE {
    static BPE: OnceLock<CoreBPE> = OnceLock::new();
    BPE.get_or_init(|| tiktoken_rs::o200k_base().expect("o200k_base"))
}

pub fn ntok(text: &str) -> usize {
    bpe().encode_ordinary(text).len()
}
