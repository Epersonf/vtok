fn main() {
    let src = std::fs::read_to_string(std::env::args().nth(1).unwrap()).unwrap();
    let t = vtok_rs::parse(src.as_bytes());
    println!("{}", t.root_node().to_sexp());
}
