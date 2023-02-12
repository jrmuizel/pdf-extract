use pdf_extract::extract_text_from_mem;

#[test]
fn extract_text() {
    let bytes = std::fs::read("tests/docs/atomic.pdf").unwrap();
    let out = extract_text_from_mem(&bytes).unwrap();
    assert!(out.contains("Atomic Data"));
}

#[test]
fn dont_panic_on_docs() {
    // For all files in docs directory, try to extract text
    for entry in std::fs::read_dir("tests/docs").unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().unwrap() == "pdf" {
            let bytes = std::fs::read(&path).unwrap();
            let _ = pdf_extract::extract_text_from_mem(&bytes)
                .unwrap_or_else(|_| panic!("Failed to extract text for {:?}", path.as_os_str()));
        }
    }
}
