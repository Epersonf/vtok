//! Tests for document nodes: PDF, images, SVG, video/audio with subtitles, docx and text documents.
use super::tests::{put, tmp};
use super::*;

fn pdf(lines: &[&str]) -> Vec<u8> {
    let mut content = String::from("BT /F1 12 Tf 72 720 Td 14 TL\n");
    for l in lines { content.push_str(&format!("({l}) Tj T*\n")); }
    content.push_str("ET");
    let objs = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>".to_string(),
        format!("<< /Length {} >>\nstream\n{content}\nendstream", content.len()),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_string(),
    ];
    let mut out = b"%PDF-1.4\n".to_vec();
    let mut offs = vec![];
    for (i, o) in objs.iter().enumerate() { offs.push(out.len()); out.extend(format!("{} 0 obj\n{o}\nendobj\n", i + 1).bytes()); }
    let xref = out.len();
    out.extend(format!("xref\n0 {}\n0000000000 65535 f \n", objs.len() + 1).bytes());
    for o in offs { out.extend(format!("{o:010} 00000 n \n").bytes()); }
    out.extend(format!("trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n", objs.len() + 1).bytes());
    out
}

fn png(w: u32, h: u32) -> Vec<u8> {
    let mut v = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
    v.extend(w.to_be_bytes());
    v.extend(h.to_be_bytes());
    v.extend([8, 2, 0, 0, 0, 0, 0, 0, 0]);
    v
}

fn code(d: &Path) { put(d, "src/lib.py", "def fetch_data():\n    return 1\n\ndef render_page():\n    return fetch_data()\n"); }

#[test]
fn pdf_becomes_a_node_with_headings_linked_to_symbols() {
    let d = tmp("pdf");
    code(&d);
    std::fs::write(d.join("spec.pdf"), pdf(&["1 Introduction", "The loader calls fetch_data() first.", "2 Rendering", "Then render_page draws it."])).unwrap();
    let g = build(&d);
    let pdf = g.nodes.iter().position(|n| n.kind == "pdf").expect("pdf node");
    assert_eq!((g.nodes[pdf].name.as_str(), g.nodes[pdf].b), ("spec.pdf", 1));
    let heads: Vec<&Node> = g.nodes.iter().filter(|n| n.kind == "page").collect();
    assert_eq!(heads.len(), 2, "{:?}", g.nodes.iter().map(|n| &n.name).collect::<Vec<_>>());
    let sym = |n: &str| g.nodes.iter().position(|x| x.name == n).unwrap();
    assert!(g.edges.contains(&(pdf, sym("fetch_data"))) && g.edges.contains(&(pdf, sym("render_page"))));
    let intro = g.nodes.iter().position(|n| n.name == "spec.pdf#1 Introduction").unwrap();
    assert!(g.edges.contains(&(intro, sym("fetch_data"))) && !g.edges.contains(&(intro, sym("render_page"))));
    assert!(g.explain("spec.pdf").contains("1 pages") && g.explain("spec.pdf").contains("[pdf]"));
    assert!(g.summary().contains("pdf 1"));
    std::fs::remove_dir_all(&d).ok();
}

#[test]
fn images_carry_dimensions_and_are_linked_from_docs_and_code() {
    let d = tmp("img");
    code(&d);
    std::fs::create_dir_all(d.join("assets")).unwrap();
    std::fs::write(d.join("assets").join("logo.png"), png(640, 480)).unwrap();
    put(&d, "assets/diagram.svg", "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"120\" height=\"80\"><title>Flow</title><text x=\"1\" y=\"2\">calls fetch_data()</text></svg>");
    put(&d, "README.md", "![logo](assets/logo.png) and ![d](assets/diagram.svg)\n");
    put(&d, "src/page.py", "def banner():\n    return open('assets/logo.png')\n");
    let g = build(&d);
    let logo = g.nodes.iter().position(|n| n.kind == "image" && n.name.ends_with("logo.png")).unwrap();
    assert_eq!((g.nodes[logo].a, g.nodes[logo].b), (640, 480));
    let svg = g.nodes.iter().position(|n| n.kind == "image" && n.name.ends_with("diagram.svg")).unwrap();
    assert_eq!((g.nodes[svg].a, g.nodes[svg].b), (120, 80));
    let from = |i: usize| -> Vec<&str> { g.edges.iter().filter(|e| e.1 == i).map(|e| g.nodes[e.0].name.as_str()).collect() };
    assert!(from(logo).contains(&"README.md") && from(logo).contains(&"banner"), "{:?}", from(logo));
    assert!(g.edges.contains(&(svg, g.nodes.iter().position(|x| x.name == "fetch_data").unwrap())), "svg text links to symbols");
    assert!(g.explain("logo.png").contains("640x480"));
    std::fs::remove_dir_all(&d).ok();
}

#[test]
fn video_audio_and_sidecar_subtitles() {
    let d = tmp("av");
    code(&d);
    // a 2 second, 8 kHz mono 8-bit wav
    let mut wav = b"RIFF\0\0\0\0WAVEfmt ".to_vec();
    wav.extend([16, 0, 0, 0, 1, 0, 1, 0, 0x40, 0x1f, 0, 0, 0x40, 0x1f, 0, 0, 1, 0, 8, 0]);
    wav.extend(b"data");
    wav.extend(16000u32.to_le_bytes());
    wav.extend(vec![128u8; 16000]);
    std::fs::write(d.join("talk.wav"), wav).unwrap();
    std::fs::write(d.join("demo.mp4"), vec![0u8; 3000]).unwrap();
    put(&d, "demo.en.srt", "1\n00:00:01,000 --> 00:00:03,000\nNow we call render_page() here\n");
    put(&d, "talk.txt", "transcript mentions fetch_data() only\n");
    let g = build(&d);
    let a = g.nodes.iter().find(|n| n.kind == "audio").unwrap();
    assert_eq!((a.name.as_str(), a.a), ("talk.wav", 2));
    let v = g.nodes.iter().position(|n| n.kind == "video").unwrap();
    assert_eq!(g.nodes[v].b, 2);
    let sub = g.nodes.iter().position(|n| n.name == "demo.en.srt").expect("subtitle doc node");
    assert!(g.edges.contains(&(v, sub)));
    assert!(g.edges.contains(&(sub, g.nodes.iter().position(|x| x.name == "render_page").unwrap())));
    assert!(g.nodes.iter().any(|n| n.name == "talk.txt" && n.kind == "doc"));
    std::fs::remove_dir_all(&d).ok();
}

#[test]
fn docx_html_and_plain_text_docs_link_to_symbols() {
    use std::io::Write;
    let d = tmp("docs");
    code(&d);
    let mut z = zip::ZipWriter::new(std::fs::File::create(d.join("design.docx")).unwrap());
    z.start_file("word/document.xml", zip::write::SimpleFileOptions::default()).unwrap();
    z.write_all(b"<w:document><w:body><w:p><w:r><w:t>The fetch_data function</w:t></w:r></w:p></w:body></w:document>").unwrap();
    z.finish().unwrap();
    put(&d, "guide.html", "<html><script>var x=render_page();</script><p>Use <b>render_page()</b> now</p></html>");
    put(&d, "notes.rst", "Call ``fetch_data`` then stop.\nUnrelated words like render are ignored.\n");
    put(&d, "plain.txt", "nothing to see here, just words\n");
    let g = build(&d);
    let doc = |n: &str| g.nodes.iter().position(|x| x.name == n && x.kind == "doc");
    let sym = |n: &str| g.nodes.iter().position(|x| x.name == n).unwrap();
    assert!(g.edges.contains(&(doc("design.docx").unwrap(), sym("fetch_data"))));
    assert!(g.edges.contains(&(doc("guide.html").unwrap(), sym("render_page"))));
    assert!(g.edges.contains(&(doc("notes.rst").unwrap(), sym("fetch_data"))));
    assert!(doc("plain.txt").is_none(), "documents that link to nothing stay out");
    let back = Graph::from_json(&g.to_json()).unwrap();
    assert_eq!(back.nodes.iter().filter(|n| n.kind == "pdf" || n.kind == "image").count(), 0);
    assert_eq!(back.nodes.len(), g.nodes.len());
    std::fs::remove_dir_all(&d).ok();
}
