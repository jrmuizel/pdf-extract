use log::info;
use pdf_extract::extract_text;
use std::io::Write;
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

#[test]
fn ignores_malformed_curve_operator() {
    let pdf = minimal_pdf_with_content(
        b"BT /F1 12 Tf 72 720 Td (Curve operand regression) Tj ET\n0 0 m\nc\nS\n",
    );
    let path = std::env::temp_dir().join(format!(
        "pdf_extract_malformed_curve_{}.pdf",
        std::process::id()
    ));
    let mut file = std::fs::File::create(&path).unwrap();
    file.write_all(&pdf).unwrap();

    let out = extract_text(&path).unwrap();
    std::fs::remove_file(&path).unwrap();
    assert!(out.contains("Curve operand regression"), "output was: {}", out);
}

fn minimal_pdf_with_content(content: &[u8]) -> Vec<u8> {
    let mut objects = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>".to_vec(),
        format!("<< /Length {} >>\nstream\n", content.len()).into_bytes(),
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_vec(),
    ];
    objects[3].extend_from_slice(content);
    objects[3].extend_from_slice(b"endstream");

    let mut pdf = b"%PDF-1.4\n".to_vec();
    let mut offsets = Vec::new();
    for (idx, object) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.extend_from_slice(format!("{} 0 obj\n", idx + 1).as_bytes());
        pdf.extend_from_slice(object);
        pdf.extend_from_slice(b"\nendobj\n");
    }

    let xref_offset = pdf.len();
    pdf.extend_from_slice(format!("xref\n0 {}\n", objects.len() + 1).as_bytes());
    pdf.extend_from_slice(b"0000000000 65535 f \n");
    for offset in offsets {
        pdf.extend_from_slice(format!("{:010} 00000 n \n", offset).as_bytes());
    }
    pdf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
            objects.len() + 1,
            xref_offset
        )
        .as_bytes(),
    );
    pdf
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
