use log::info;
use lopdf::{Dictionary, Document, Object, Stream};
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

#[test]
fn empty_operand_operators_are_skipped() {
    // Craft a minimal in-memory PDF whose page content stream emits CS and w
    // with zero operands. Both operators need one operand, and before the
    // guard this made pdf-extract panic in `process_stream` with
    // "index out of bounds: the len is 0 but the index is 0". The fix must
    // skip such malformed operators and return Ok instead of aborting.
    let mut doc = Document::new();

    // Bare operators with missing operands: CS (colorspace) and w (line width).
    let content = Stream::new(Dictionary::new(), b"BT\nCS\nw\nET\n".to_vec());
    let content_id = doc.add_object(content);

    let mut page = Dictionary::new();
    page.set("Type", Object::Name(b"Page".to_vec()));
    page.set("MediaBox", Object::Array(vec![
        Object::Integer(0),
        Object::Integer(0),
        Object::Integer(612),
        Object::Integer(792),
    ]));
    page.set("Contents", Object::Reference(content_id));
    page.set("Resources", Object::Dictionary(Dictionary::new()));
    let page_id = doc.add_object(Object::Dictionary(page));

    let mut pages = Dictionary::new();
    pages.set("Type", Object::Name(b"Pages".to_vec()));
    pages.set("Kids", Object::Array(vec![Object::Reference(page_id)]));
    pages.set("Count", Object::Integer(1));
    let pages_id = doc.add_object(Object::Dictionary(pages));

    let mut catalog = Dictionary::new();
    catalog.set("Type", Object::Name(b"Catalog".to_vec()));
    catalog.set("Pages", Object::Reference(pages_id));
    let catalog_id = doc.add_object(Object::Dictionary(catalog));
    doc.trailer.set("Root", Object::Reference(catalog_id));

    let mut bytes = Vec::new();
    doc.save_to(&mut bytes).expect("serialize crafted PDF");

    let out = pdf_extract::extract_text_from_mem(&bytes)
        .unwrap_or_else(|e| panic!("extract_text_from_mem failed on crafted PDF: {}", e));
    assert!(out.is_empty(), "expected no text, got {:?}", out);
}
