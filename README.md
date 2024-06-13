## pdf-extract

[![Build Status](https://github.com/jrmuizel/pdf-extract/actions/workflows/rust.yml/badge.svg)](https://github.com/jrmuizel/pdf-extract/actions)
[![crates.io](https://img.shields.io/crates/v/pdf-extract.svg)](https://crates.io/crates/pdf-extract)
[![Documentation](https://docs.rs/pdf-extract/badge.svg)](https://docs.rs/pdf-extract)

A rust library to extract content from PDF files.

```rust
let bytes = std::fs::read("tests/docs/simple.pdf").unwrap();
let out = pdf_extract::extract_text_from_mem(&bytes).unwrap();
assert!(out.contains("This is a small demonstration"));
```

## Run Example

```sh
RUST_LOG=info cargo run --example extract <pdf-filename>
```

## See also

- https://github.com/elacin/PDFExtract/
- https://github.com/euske/pdfminer
- https://github.com/CrossRef/pdfextract
- https://github.com/VikParuchuri/marker
- https://github.com/kermitt2/pdfalto used by [grobid](https://github.com/kermitt2/grobid/)

### Not PDF specific

- https://github.com/Layout-Parser/layout-parser
