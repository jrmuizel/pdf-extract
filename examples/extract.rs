extern crate pdf_extract;

use std::env;
use std::path::PathBuf;
use std::path;
use std::io::BufWriter;
use std::fs::File;
use std::sync::Arc;
use pdf_extract::*;
use hayro_syntax::Pdf;
use simple_logger::SimpleLogger;

fn main() {
    SimpleLogger::new().init().unwrap();
    let file = env::args().nth(1).unwrap();
    let output_kind = env::args().nth(2).unwrap_or_else(|| "txt".to_owned());
    println!("{}", file);
    let path = path::Path::new(&file);
    let filename = path.file_name().expect("expected a filename");
    let mut output_file = PathBuf::new();
    output_file.push(filename);
    output_file.set_extension(&output_kind);
    let mut output_file = BufWriter::new(File::create(output_file).expect("could not create output"));
    let data = std::fs::read(path).unwrap();
    let pdf = Pdf::new(Arc::new(data)).unwrap();

    print_metadata(&pdf);

    let mut output: Box<dyn OutputDev> = match output_kind.as_ref() {
        "txt" => Box::new(PlainTextOutput::new(&mut output_file as &mut dyn std::io::Write)),
        "html" => Box::new(HTMLOutput::new(&mut output_file)),
        "svg" => Box::new(SVGOutput::new(&mut output_file)),
        _ => panic!(),
    };

    output_doc(&pdf, output.as_mut()).unwrap();
}
