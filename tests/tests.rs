use pdf_extract::{extract_text, extract_text_from_mem};

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
    let docs = vec![
        expected!("complex.pdf", "communicate state changes"),
        expected!("simple.pdf", "And more text"),
        expected!("version1_2.pdf", "HERE IS ALL CAPS"),
        expected!("version1_3.pdf", "HERE IS ALL CAPS"),
        expected!("from_macos_pages.pdf", "hope this works"),
    ];
    ExpectedText::test(&docs);
}

#[test]
fn extract_mem() {
    let bytes = std::fs::read("tests/docs/complex.pdf").unwrap();
    let out = extract_text_from_mem(&bytes).unwrap();
    assert!(out.contains("Atomic Data"), "Text not correctly extracted");
}

#[test]
fn extract_from_path() {
    let path = "tests/docs/complex.pdf";
    let out = extract_text(path).unwrap();
    assert!(out.contains("Atomic Data"), "Text not correctly extracted");
}

#[test]
fn dont_panic_on_docs() {
    // For all files in docs directory, try to extract text
    for entry in std::fs::read_dir("tests/docs").unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().unwrap() == "pdf" {
            let bytes = std::fs::read(&path).unwrap();
            let filename = path.as_os_str().to_str().unwrap();
            let out = extract_text_from_mem(&bytes)
                .unwrap_or_else(|_| panic!("Failed to extract text for {}", filename));
            assert!(!out.is_empty(), "No text extracted for {}", filename);
        } else {
            panic!("only .pdf files are allowed in /docs")
        }
    }
}

// data structure to make it easy to check if certain files are correctly parsed
// e.g. ExpectedText { filename: "atomic.pdf", text: "Atomic Data" }
#[derive(Debug, PartialEq)]
struct ExpectedText {
    filename: &'static str,
    text: &'static str,
}

impl ExpectedText {
    fn test(expected: &[ExpectedText]) {
        for ExpectedText { filename, text } in expected {
            let path = format!("tests/docs/{}", filename);
            let out = extract_text(path).unwrap();
            assert!(out.contains(text), "Text not correctly extracted");
        }
    }
}
