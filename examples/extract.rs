extern crate pdf_extract;

use pdf_extract::*;
use std::env;
use std::fs::File;
use std::io::BufWriter;
use std::path;
use std::path::PathBuf;

/// Parses the first CLI argument as a PDF file and outputs the text to a `.txt` file in this directory.
fn main() {
    //let output_kind = "html";
    //let output_kind = "txt";
    //let output_kind = "svg";
    let file = env::args().nth(1).expect("expected a filename");
    let output_kind = env::args().nth(2).unwrap_or_else(|| "txt".to_owned());
    let path = path::Path::new(&file);
    let filename = path.file_name().expect("expected a filename");
    let mut output_path = PathBuf::new();
    output_path.push(filename);
    output_path.set_extension(&output_kind);
    println!("output file: {:?}", output_path.as_os_str());
    let mut output_file =
        BufWriter::new(File::create(output_path).expect("could not create output"));
    let doc = Document::load(path).unwrap();

    print_metadata(&doc).unwrap();

    let mut output: Box<dyn OutputDev> = match output_kind.as_ref() {
        "txt" => Box::new(PlainTextOutput::new(
            &mut output_file as &mut dyn std::io::Write,
        )),
        "html" => Box::new(HTMLOutput::new(&mut output_file)),
        "svg" => Box::new(SVGOutput::new(&mut output_file)),
        _ => panic!(),
    };

    output_doc(&doc, output.as_mut()).unwrap();
}
