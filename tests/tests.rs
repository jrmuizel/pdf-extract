use log::info;
use pdf_extract::extract_text;
use test_log::test;
// Shorthand for creating ExpectedText
// example: expected!("atomic.pdf", "Atomic Data");
macro_rules! expected {
    ($filename:expr, $text:expr) => {
        ExpectedText {
            filename: $filename,
            text: $text,
        }
    };
}

// Use the macro to create a list of ExpectedText
// and then check if the text is correctly extracted
#[test]
fn extract_expected_text() {
    let docs = vec![expected!("documents_stack.pdf.link", "mouse button until")];
    for doc in docs {
        doc.test();
    }
}

#[test]
// iterate over all docs in the `tests/docs` directory, don't crash
fn extract_all_docs() {
    let docs = std::fs::read_dir("tests/docs").unwrap();
    for doc in docs {
        let doc = doc.unwrap();
        let path = doc.path();
        let filename = path.file_name().unwrap().to_string_lossy();
        expected!(&filename, "").test();
    }
}

// data structure to make it easy to check if certain files are correctly parsed
// e.g. ExpectedText { filename: "atomic.pdf", text: "Atomic Data" }
#[derive(Debug, PartialEq)]
struct ExpectedText<'a> {
    filename: &'a str,
    text: &'a str,
}

impl ExpectedText<'_> {
    /// Opens the `filename` from `tests/docs`, extracts the text and checks if it contains `text`
    /// If the file ends with `_link`, it will download the file from the url in the file to the `tests/docs_cache` directory
    fn test(self) {
        let ExpectedText { filename, text } = self;
        let file_path = if filename.ends_with(".pdf.link") {
            let docs_cache = "tests/docs_cache";
            if !std::path::Path::new(docs_cache).exists() {
                // This might race with exists test above, but that's fine
                if let Err(e) = std::fs::create_dir(docs_cache) {
                    if e.kind() != std::io::ErrorKind::AlreadyExists {
                        panic!("Failed to create directory {}, {}", docs_cache, e);
                    }
                } 
            }
            let file_path = format!("{}/{}", docs_cache, filename.replace(".link", ""));
            if std::path::Path::new(&file_path).exists() {
                file_path
            } else {
                let url = std::fs::read_to_string(format!("tests/docs/{}", filename)).unwrap();
                let resp = ureq::get(&url).call().unwrap();
                let mut file = std::fs::File::create(&file_path).unwrap();
                std::io::copy(&mut resp.into_reader(), &mut file).unwrap();
                file_path
            }
        } else {
            format!("tests/docs/{}", filename)
        };
        let out = extract_text(file_path)
            .unwrap_or_else(|e| panic!("Failed to extract text from {}, {}", filename, e));
        info!("{}", out);
        assert!(
            out.contains(text),
            "Text {} does not contain '{}'",
            filename,
            text
        );
    }
}

/// A one-line page in a CID font whose digits' widths use the range form
/// `c_first c_last w`, as Chrome writes them, followed by a run placed with
/// Td at the true end of the first run (the way kerned text is laid out).
fn cid_range_widths_pdf(first: &str, second: &str) -> Vec<u8> {
    use pdf_extract::content::{Content, Operation};
    use pdf_extract::{dictionary, Document, Object, Stream, StringFormat};

    let width = |c: char| -> i64 {
        match c {
            '0'..='9' | 'L' => 556,
            ' ' => 278,
            'P' => 667,
            'T' => 611,
            other => panic!("no width for {other:?}"),
        }
    };
    let mut used: Vec<char> = first.chars().chain(second.chars()).collect();
    used.sort_unstable();
    used.dedup();
    // digits as one range, every other glyph in the array form
    let mut w: Vec<Object> = vec![48.into(), 57.into(), 556.into()];
    for c in used.iter().filter(|c| !c.is_ascii_digit()) {
        w.push((*c as i64).into());
        w.push(vec![Object::from(width(*c))].into());
    }
    let bfchars: String = used
        .iter()
        .map(|c| format!("<{:04X}> <{:04X}>\n", *c as u32, *c as u32))
        .collect();
    let cmap = format!(
        "/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n\
         /CMapName /Adobe-Identity-UCS def\n/CMapType 2 def\n\
         1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n\
         {} beginbfchar\n{bfchars}endbfchar\nendcmap\n\
         CMapName currentdict /CMap defineresource pop\nend\nend\n",
        used.len()
    );

    let mut doc = Document::with_version("1.5");
    let pages_id = doc.new_object_id();
    let to_unicode = doc.add_object(Stream::new(dictionary! {}, cmap.into_bytes()));
    let descriptor = doc.add_object(dictionary! {
        "Type" => "FontDescriptor",
        "FontName" => "AAAAAA+ArialMT",
        "Flags" => 32,
        "FontBBox" => vec![0.into(), (-200).into(), 1000.into(), 900.into()],
        "ItalicAngle" => 0,
        "Ascent" => 900,
        "Descent" => -200,
        "CapHeight" => 700,
        "StemV" => 80,
    });
    let cid_font = doc.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "CIDFontType2",
        "BaseFont" => "AAAAAA+ArialMT",
        "CIDSystemInfo" => dictionary! {
            "Registry" => Object::string_literal("Adobe"),
            "Ordering" => Object::string_literal("Identity"),
            "Supplement" => 0,
        },
        "FontDescriptor" => descriptor,
        "DW" => 500,
        "W" => w,
        "CIDToGIDMap" => "Identity",
    });
    let font = doc.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type0",
        "BaseFont" => "AAAAAA+ArialMT",
        "Encoding" => "Identity-H",
        "DescendantFonts" => vec![cid_font.into()],
        "ToUnicode" => to_unicode,
    });
    let resources = doc.add_object(dictionary! { "Font" => dictionary! { "F1" => font } });
    let codes = |s: &str| -> Object {
        let bytes: Vec<u8> = s.chars().flat_map(|c| (c as u16).to_be_bytes()).collect();
        Object::String(bytes, StringFormat::Hexadecimal)
    };
    // font size 10: an advance of 1000 thousandths of an em is 10 units
    let advance = first.chars().map(width).sum::<i64>() as f32 / 100.;
    let content = Content {
        operations: vec![
            Operation::new("BT", vec![]),
            Operation::new("Tf", vec!["F1".into(), 10.into()]),
            Operation::new("Tm", vec![1.into(), 0.into(), 0.into(), 1.into(), 72.into(), 700.into()]),
            Operation::new("Tj", vec![codes(first)]),
            Operation::new("Td", vec![advance.into(), 0.into()]),
            Operation::new("Tj", vec![codes(second)]),
            Operation::new("ET", vec![]),
        ],
    };
    let contents = doc.add_object(Stream::new(dictionary! {}, content.encode().unwrap()));
    let page = doc.add_object(dictionary! {
        "Type" => "Page",
        "Parent" => pages_id,
        "Contents" => contents,
        "Resources" => resources,
        "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
    });
    doc.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! { "Type" => "Pages", "Kids" => vec![page.into()], "Count" => 1 }),
    );
    let catalog = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
    doc.trailer.set("Root", catalog);
    let mut bytes = Vec::new();
    doc.save_to(&mut bytes).unwrap();
    bytes
}

#[test]
fn cid_width_ranges_give_their_glyphs_the_range_width() {
    // with the digits read at DW (500) instead of 556, the second run starts
    // past where the first one ends and reads as a new word: "649 PL T"
    let bytes = cid_range_widths_pdf("649 PL", "T");
    let out = pdf_extract::extract_text_from_mem(&bytes).unwrap();
    assert_eq!(out.trim(), "649 PLT");
}
