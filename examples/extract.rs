extern crate pdf_extract;
extern crate lopdf;

use std::fmt::Debug;
use std::env;

use std::fmt;
use std::path::PathBuf;
use std::path;
use std::io::Write;
use std::str;
use std::fs::File;
use std::slice::Iter;
use std::collections::HashMap;
use std::rc::Rc;
use pdf_extract::*;
use lopdf::*;

fn main() {
    //let output_kind = "html";
    let output_kind = "txt";
    //let output_kind = "svg";
    let file = env::args().nth(1).unwrap();
    println!("{}", file);
    let path = path::Path::new(&file);
    let filename = path.file_name().expect("expected a filename");
    let mut output_file = PathBuf::new();
    output_file.push(filename);
    output_file.set_extension(output_kind);
    let mut output_file = File::create(output_file).expect("could not create output");
    let doc = Document::load(path).unwrap();

    print_metadata(&doc);

    let mut output: Box<OutputDev> = match output_kind {
        "txt" => Box::new(PlainTextOutput::new(&mut output_file)),
        "html" => Box::new(HTMLOutput::new(&mut output_file)),
        "svg" => Box::new(SVGOutput::new(&mut output_file)),
        _ => panic!(),
    };

    output_doc(&doc, &mut *output);
}