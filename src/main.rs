extern crate lopdf;
use lopdf::Document;
use lopdf::Dictionary;
use lopdf::{Object, ObjectId, Stream};
use lopdf::StringFormat;
use std::env;
extern crate flate2;
extern crate encoding;
extern crate pom;
use encoding::{Encoding, DecoderTrap};
use encoding::all::UTF_16BE;


fn get_info(doc: &Document) -> Option<&Dictionary> {
    match doc.trailer.get("Info") {
        Some(&Object::Reference(ref id)) => {
            match doc.get_object(*id) {
                Some(&Object::Dictionary(ref info)) => { return Some(info); }
                _ => {}
            }
        }
        _ => {}
    }
    None
}

fn get_catalog(doc: &Document) -> &Dictionary {
    match doc.trailer.get("Root").unwrap() {
        &Object::Reference(ref id) => {
            match doc.get_object(*id) {
                Some(&Object::Dictionary(ref catalog)) => { return catalog; }
                _ => {}
            }
        }
        _ => {}
    }
    panic!();
}

fn get_pages(doc: &Document) -> &Dictionary {
    let catalog = get_catalog(doc);
    match catalog.get("Pages").unwrap() {
        &Object::Reference(ref id) => {
            match doc.get_object(*id) {
                Some(&Object::Dictionary(ref pages)) => { return pages; }
                _ => {}
            }
        }
        _ => {}
    }
    panic!();
}

#[allow(non_upper_case_globals)]
const PDFDocEncoding: &'static [u16] = &[
    0x0000, 0x0001, 0x0002, 0x0003, 0x0004, 0x0005, 0x0006, 0x0007, 0x0008,
    0x0009, 0x000a, 0x000b, 0x000c, 0x000d, 0x000e, 0x000f, 0x0010, 0x0011,
    0x0012, 0x0013, 0x0014, 0x0015, 0x0016, 0x0017, 0x02d8, 0x02c7, 0x02c6,
    0x02d9, 0x02dd, 0x02db, 0x02da, 0x02dc, 0x0020, 0x0021, 0x0022, 0x0023,
    0x0024, 0x0025, 0x0026, 0x0027, 0x0028, 0x0029, 0x002a, 0x002b, 0x002c,
    0x002d, 0x002e, 0x002f, 0x0030, 0x0031, 0x0032, 0x0033, 0x0034, 0x0035,
    0x0036, 0x0037, 0x0038, 0x0039, 0x003a, 0x003b, 0x003c, 0x003d, 0x003e,
    0x003f, 0x0040, 0x0041, 0x0042, 0x0043, 0x0044, 0x0045, 0x0046, 0x0047,
    0x0048, 0x0049, 0x004a, 0x004b, 0x004c, 0x004d, 0x004e, 0x004f, 0x0050,
    0x0051, 0x0052, 0x0053, 0x0054, 0x0055, 0x0056, 0x0057, 0x0058, 0x0059,
    0x005a, 0x005b, 0x005c, 0x005d, 0x005e, 0x005f, 0x0060, 0x0061, 0x0062,
    0x0063, 0x0064, 0x0065, 0x0066, 0x0067, 0x0068, 0x0069, 0x006a, 0x006b,
    0x006c, 0x006d, 0x006e, 0x006f, 0x0070, 0x0071, 0x0072, 0x0073, 0x0074,
    0x0075, 0x0076, 0x0077, 0x0078, 0x0079, 0x007a, 0x007b, 0x007c, 0x007d,
    0x007e, 0x0000, 0x2022, 0x2020, 0x2021, 0x2026, 0x2014, 0x2013, 0x0192,
    0x2044, 0x2039, 0x203a, 0x2212, 0x2030, 0x201e, 0x201c, 0x201d, 0x2018,
    0x2019, 0x201a, 0x2122, 0xfb01, 0xfb02, 0x0141, 0x0152, 0x0160, 0x0178,
    0x017d, 0x0131, 0x0142, 0x0153, 0x0161, 0x017e, 0x0000, 0x20ac, 0x00a1,
    0x00a2, 0x00a3, 0x00a4, 0x00a5, 0x00a6, 0x00a7, 0x00a8, 0x00a9, 0x00aa,
    0x00ab, 0x00ac, 0x0000, 0x00ae, 0x00af, 0x00b0, 0x00b1, 0x00b2, 0x00b3,
    0x00b4, 0x00b5, 0x00b6, 0x00b7, 0x00b8, 0x00b9, 0x00ba, 0x00bb, 0x00bc,
    0x00bd, 0x00be, 0x00bf, 0x00c0, 0x00c1, 0x00c2, 0x00c3, 0x00c4, 0x00c5,
    0x00c6, 0x00c7, 0x00c8, 0x00c9, 0x00ca, 0x00cb, 0x00cc, 0x00cd, 0x00ce,
    0x00cf, 0x00d0, 0x00d1, 0x00d2, 0x00d3, 0x00d4, 0x00d5, 0x00d6, 0x00d7,
    0x00d8, 0x00d9, 0x00da, 0x00db, 0x00dc, 0x00dd, 0x00de, 0x00df, 0x00e0,
    0x00e1, 0x00e2, 0x00e3, 0x00e4, 0x00e5, 0x00e6, 0x00e7, 0x00e8, 0x00e9,
    0x00ea, 0x00eb, 0x00ec, 0x00ed, 0x00ee, 0x00ef, 0x00f0, 0x00f1, 0x00f2,
    0x00f3, 0x00f4, 0x00f5, 0x00f6, 0x00f7, 0x00f8, 0x00f9, 0x00fa, 0x00fb,
    0x00fc, 0x00fd, 0x00fe, 0x00ff];

fn to_utf8(s: &Vec<u8>) -> String {
    if s.len() > 2 && s[0] == 0xfe && s[1] == 0xff {
        return UTF_16BE.decode(&s[2..], DecoderTrap::Strict).unwrap()
    } else {
        let r : Vec<u8> = s.iter().map(|x| *x).flat_map(|x| {
               let k = PDFDocEncoding[x as usize];
               vec![(k>>8) as u8, k as u8].into_iter()}).collect();
        return UTF_16BE.decode(&r, DecoderTrap::Strict).unwrap()
    }
}


struct Pages<'a> {
    index: usize,
    cur: &'a Dictionary,
    cur_id: ObjectId,
    doc: &'a Document,
}

impl<'a> Pages<'a> {
    fn new(doc: &Document) -> Pages {
        let catalog = get_catalog(doc);
        let mut parent_id = catalog.get("Pages").unwrap().as_reference().unwrap();
        let mut parent = match doc.get_object(parent_id) {
                Some(&Object::Dictionary(ref pages)) => { pages }
                _ => { panic!() }
            };
        loop {
            let kids = parent.get("Kids").and_then(|x| x.as_array()).unwrap();
            let child_id = kids[0].as_reference().unwrap();
            let child = doc.get_object(child_id).and_then(
                                                               |x| match x {
                                                               &Object::Dictionary(ref dict) => { Some(dict) }
                                                               _ => { None }}).unwrap();
            if get_type(child) == "Page" {
                return Pages {cur: parent, index: 0, cur_id: parent_id, doc: doc }
            }
            assert!(get_type(child) == "Pages");
            parent_id = child_id;
            parent = child;
        }

    }
}

impl<'a> Iterator for Pages<'a> {
    type Item = &'a Dictionary;

    fn next(&mut self) -> Option<&'a Dictionary> {
        let mut kids = self.cur.get("Kids").and_then(|x| x.as_array()).unwrap();
        if self.index >= kids.len() {
            println!("moving up tree");
            let mut id = self.cur_id;
            loop {
                if self.cur.get("Parent").is_none() {
                    return None;
                }
                let mut parent_id = self.cur.get("Parent").unwrap().as_reference().unwrap();
                let mut parent = self.doc.get_object(parent_id).and_then(
                                                                     |x| match x {
                                                                     &Object::Dictionary(ref dict) => { Some(dict) }
                                                                     _ => { None }});
                if let Some(ref parent) = parent {
                    let mut kids = parent.get("Kids").and_then(|x| x.as_array()).unwrap();
                    let mut parent_index = 0;
                    for kid in kids {
                        parent_index += 1;
                        if kid.as_reference().unwrap() == id {
                            break;
                        }
                    }
                    println!("parent_index {:?}", parent_index);
                    if parent_index < kids.len() {
                        // descend as deep as possible
                        let mut parent = *parent;
                        loop {
                            let child_id = kids[parent_index].as_reference().unwrap();
                            let child = self.doc.get_object(child_id).and_then(
                                                                               |x| match x {
                                                                               &Object::Dictionary(ref dict) => { Some(dict) }
                                                                               _ => { None }}).unwrap();
                            if get_type(child) == "Page" {
                                println!("found page {:?}", parent);
                                self.index = 0;
                                self.cur_id = parent_id;
                                self.cur = parent;
                                break;
                            }
                            assert!(get_type(child) == "Pages");
                            parent_id = child_id;
                            parent_index = 0;
                            parent = child;
                            kids = child.get("Kids").unwrap().as_array().unwrap();
                        }
                        break;
                    } else {
                        id = parent_id;
                        self.cur = parent;
                    }
                } else {
                    return None;
                }
            }
        }
        self.index += 1;
        let mut kids = self.cur.get("Kids").and_then(|x| x.as_array()).unwrap();
        return Some(self.doc.get_object(kids[self.index-1].as_reference().unwrap()).and_then(
                                                                                              |x| match x {
                                                                                              &Object::Dictionary(ref dict) => { Some(dict) }
                                                                                              _ => { None }}).unwrap());
    }
}

fn get_type(o: &Dictionary) -> &str
{
    o.get("Type").and_then(|x| x.as_name()).unwrap()
}

fn filter_data(contents: &Stream) -> Vec<u8> {
    use std::io::prelude::*;
    use flate2::read::ZlibDecoder;

    let mut data = Vec::new();
    if let Some(filter) = contents.filter() {
        match filter.as_str() {
            "FlateDecode" => {
                {
                    let mut decoder = ZlibDecoder::new(contents.content.as_slice());
                    decoder.read_to_end(&mut data).unwrap();
                }
            },
                _ => { data = contents.content.clone(); }
        }
    }
    data
}

struct TextState<'a>
{
    font: &'a str,
    size: f64
}

fn process_stream(contents: &Stream) {
    let data = filter_data(contents);
    let content = contents.decode_content(&data).unwrap();
    let mut ts = TextState { font:"", size:3. };
    for operation in &content.operations {
        match operation.operator.as_ref() {
            "TJ" => {
                match operation.operands[0] {
                    Object::Array(ref array) => {
                        for e in array {
                            match e {
                                &Object::String(ref s, StringFormat::Literal) => {
                                    println!("{} {:?}", ts.font, to_utf8(s));
                                }
                                _ => {}
                            }
                        }
                    }
                    _ => {}
                }
            }
            "Tj" => {
                match operation.operands[0] {
                    Object::String(ref s, StringFormat::Literal) => {
                        println!("{:?}", to_utf8(s));
                    }
                    _ => {}
                }
            }
            "Tf" => {
                ts.font = operation.operands[0].as_name().unwrap();
                match operation.operands[1] {
                    Object::Real(size) => { ts.size = size }
                    _ => {}
                }
            }


            _ => { println!("{:?}", operation);}
        }
    }
}


fn main() {
    let file = env::args().nth(1).unwrap();
    println!("{}", file);
    let doc = Document::load(file).unwrap();
    println!("Version: {}", doc.version);
    if let Some(ref info) = get_info(&doc) {
        for (k, v) in *info {
            match v {
                &Object::String(ref s, StringFormat::Literal) => { println!("{}: {}", k, to_utf8(s)); }
                _ => {}
            }
        }
    }
    println!("Page count: {}", get_pages(&doc).get("Count").unwrap().as_i64().unwrap());
    println!("Pages: {:?}", get_pages(&doc));
    println!("Type: {:?}", get_pages(&doc).get("Type").and_then(|x| x.as_name()).unwrap());
    for dict in Pages::new(&doc) {
        println!("page {:?}", dict);
        println!("resources {:?}", doc.get_object(dict.get("Resources").unwrap().as_reference().unwrap()).unwrap().as_dict());
        // Contents can point to either an array of references or a single reference
        match dict.get("Contents") {
            Some(&Object::Reference(ref id)) => {
                match doc.get_object(*id).unwrap() {
                    &Object::Stream(ref contents) => {
                        process_stream(contents);
                    }
                    _ => {}
                }
            }
            Some(&Object::Array(ref arr)) => {
                for id in arr {
                    let id = id.as_reference().unwrap();
                    match doc.get_object(id).unwrap() {
                        &Object::Stream(ref contents) => {
                            process_stream(contents);
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
}
