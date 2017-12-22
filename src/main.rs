extern crate lopdf;

use lopdf::content::Content;
use lopdf::*;
use std::fmt::Debug;
use std::env;
extern crate encoding;
extern crate euclid;
extern crate adobe_cmap_parser;
extern crate type1_encoding_parser;
use encoding::{Encoding, DecoderTrap};
use encoding::all::UTF_16BE;
use std::fmt;
use std::path::{Path, PathBuf};
use std::io::Write;
use std::str;
use std::fs::File;
use std::slice::Iter;
use std::collections::HashMap;
use std::rc::Rc;
mod metrics;
mod glyphnames;
mod encodings;

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
                other => {println!("pages: {:?}", other)}
            }
        }
        other => { println!("pages: {:?}", other)}
    }
    println!("catalog {:?}", catalog);
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

fn pdf_to_utf8(s: &[u8]) -> String {
    if s.len() > 2 && s[0] == 0xfe && s[1] == 0xff {
        return UTF_16BE.decode(&s[2..], DecoderTrap::Strict).unwrap()
    } else {
        let r : Vec<u8> = s.iter().map(|x| *x).flat_map(|x| {
               let k = PDFDocEncoding[x as usize];
               vec![(k>>8) as u8, k as u8].into_iter()}).collect();
        return UTF_16BE.decode(&r, DecoderTrap::Strict).unwrap()
    }
}

fn to_utf8(encoding: &[u16], s: &[u8]) -> String {
    if s.len() > 2 && s[0] == 0xfe && s[1] == 0xff {
        return UTF_16BE.decode(&s[2..], DecoderTrap::Strict).unwrap()
    } else {
        let r : Vec<u8> = s.iter().map(|x| *x).flat_map(|x| {
            let k = encoding[x as usize];
            vec![(k>>8) as u8, k as u8].into_iter()}).collect();
        return UTF_16BE.decode(&r, DecoderTrap::Strict).unwrap()
    }
}



fn get_type(o: &Dictionary) -> &str
{
    o.type_name().unwrap()
}


fn get_obj<'a>(doc: &'a Document, o: &Object) -> &'a Object
{
    doc.get_object(o.as_reference().unwrap()).unwrap()
}

fn maybe_deref<'a>(doc: &'a Document, o: &'a Object) -> &'a Object {
    match o {
        &Object::Reference(r) => doc.get_object(r).expect("missing object reference"),
        _ => o
    }
}

fn maybe_get_obj<'a>(doc: &'a Document, dict: &'a Dictionary, key: &str) -> Option<&'a Object> {
    dict.get(key).map(|o| maybe_deref(doc, o))
}

fn get_name<'a>(doc: &'a Document, dict: &'a Dictionary, key: &str) -> String {
    pdf_to_utf8(dict.get(key).map(|o| maybe_deref(doc, o)).unwrap().as_name().unwrap())
}

fn maybe_get_name<'a>(doc: &'a Document, dict: &'a Dictionary, key: &str) -> Option<String> {
    maybe_get_obj(doc, dict, key).and_then(|n| n.as_name()).map(|n| pdf_to_utf8(n))
}

fn maybe_get_array<'a>(doc: &'a Document, dict: &'a Dictionary, key: &str) -> Option<&'a Vec<Object>> {
    maybe_get_obj(doc, dict, key).and_then(|n| n.as_array())
}

#[derive(Clone)]
struct PdfBasicFont<'a> {
    font: &'a Dictionary,
    doc: &'a Document,
    encoding: Option<Vec<u16>>,
    unicode_map: Option<HashMap<u32, u32>>,
    widths: HashMap<i64, f64>, // should probably just use i32 here
    default_width: Option<f64>, // only used for CID fonts and we should probably brake out the different font types
}




fn make_font<'a>(doc: &'a Document, font: &'a Dictionary) -> Rc<PdfFont + 'a> {
    let subtype = get_name(doc, font, "Subtype");
    println!("MakeFont({})", subtype);
    if subtype == "Type0" {
        Rc::new(PdfCIDFont::new(doc, font))
    } else {
        Rc::new(PdfBasicFont::new(doc, font))
    }
}

impl<'a> PdfBasicFont<'a> {
    fn new(doc: &'a Document, font: &'a Dictionary) -> PdfBasicFont<'a> {
        let base_name = get_name(doc, font, "BaseFont");
        let subtype = get_name(doc, font, "Subtype");

        let encoding = maybe_get_obj(doc, font, "Encoding");
        println!("base_name {} {} enc:{:?} {:?}", base_name, subtype, encoding, font);
        let descriptor = maybe_get_obj(doc, font, "FontDescriptor").and_then(|x| x.as_dict());
        let mut type1_encoding = None;
        if let Some(descriptor) = descriptor {
            println!("descriptor {:?}", descriptor);
            let file = maybe_get_obj(doc, descriptor, "FontFile");
            if subtype == "Type1" {
                match file {
                    Some(&Object::Stream(ref s)) => {
                        let s = get_contents(s);
                        //println!("font contents {:?}", pdf_to_utf8(&s));
                        type1_encoding = Some(type1_encoding_parser::get_encoding_map(&s).expect("encoding"));

                    }
                    _ => { println!("font file {:?}", file) }
                }
            }

            let charset = maybe_get_obj(doc, descriptor, "CharSet");
            let charset = match charset {
                Some(&Object::String(ref s, _)) => { Some(pdf_to_utf8(&s)) }
                _ => { None }
            };
            //println!("charset {:?}", charset);
        }

        let unicode_map = get_unicode_map(doc, font);

        let mut encoding_table = None;
        if let Some(encoding) = encoding {
            if let &Object::Name(ref encoding_name) = encoding {
                println!("encoding {:?}", pdf_to_utf8(encoding_name));
                //encoding_table = Some(Vec::from(
                let encoding = match &encoding_name[..] {
                    b"MacRomanEncoding" => encodings::MAC_ROMAN_ENCODING,
                    b"MacExpertEncoding" => encodings::MAC_EXPERT_ENCODING,
                    b"WinAnsiEncoding" => encodings::WIN_ANSI_ENCODING,
                    _ => panic!("unexpected encoding {:?}", pdf_to_utf8(encoding_name))
                };
                encoding_table = Some(encoding.iter()
                    .map(|x| if let &Some(x) = x { glyphnames::name_to_unicode(x).unwrap() } else { 0 })
                    .collect());

            } else {
                let encoding = encoding.as_dict().unwrap();
                //println!("Encoding {:?}", encoding);
                if let Some(base_encoding) = maybe_get_obj(doc, encoding, "BaseEncoding") {
                    panic!("BaseEncoding {:?}", base_encoding);
                }
                let mut table = Vec::from(PDFDocEncoding);
                let differences = maybe_get_array(doc, encoding, "Differences");
                if let Some(differences) = differences {
                    let mut code = 0;
                    for o in differences {
                        match o {
                            &Object::Integer(i) => { code = i; },
                            &Object::Name(ref n) => {
                                let name = pdf_to_utf8(&n);
                                let unicode = glyphnames::name_to_unicode(&name).unwrap();
                                table[code as usize] = unicode;
                                code += 1;
                            }
                            _ => { panic!("wrong type"); }
                        }
                    }
                }
                let name = pdf_to_utf8(encoding.get("Type").unwrap().as_name().unwrap());
                println!("name: {}", name);

                encoding_table = Some(table);
            }
        } else {
            if let Some(type1_encoding) = type1_encoding {
                let mut table = Vec::from(PDFDocEncoding);
                println!("type1encoding");
                for (code, name) in type1_encoding {
                    let unicode = glyphnames::name_to_unicode(&pdf_to_utf8(&name));
                    if let Some(unicode) = unicode {
                        table[code as usize] = unicode;
                    } else {
                        println!("unknown character {}", pdf_to_utf8(&name));
                    }
                }
                encoding_table = Some(table)
            }
        }

        let mut width_map = HashMap::new();
        if base_name == "Times-Roman" {
            for font_metrics in metrics::metrics() {
                if font_metrics.0 == base_name {
                    for w in font_metrics.1 {
                        width_map.insert(w.0, w.1 as f64);
                    }
                    assert!(maybe_get_obj(doc, font, "FirstChar").is_none());
                    assert!(maybe_get_obj(doc, font, "LastChar").is_none());
                    assert!(maybe_get_obj(doc, font, "Widths").is_none());
                }
            }
        } else {
            let mut first_char;
            let mut last_char;
            let mut widths;
            first_char = maybe_get_obj(doc, font, "FirstChar").and_then(|fc| fc.as_i64()).expect("missing FirstChar");
            last_char = maybe_get_obj(doc, font, "LastChar").and_then(|fc| fc.as_i64()).expect("missing LastChar");
            widths = maybe_get_obj(doc, font, "Widths").and_then(|widths| widths.as_array()).expect("Widths should be an array")
                .iter()
                .map(|width| as_num(width))
                .collect::<Vec<_>>();
            let mut i = 0;
            println!("first_char {:?}, last_char: {:?}, widths: {} {:?}", first_char, last_char, widths.len(), widths);

            for w in widths {
                width_map.insert((first_char + i), w);
                i += 1;
            }
            assert_eq!(first_char + i - 1, last_char);
        }

        PdfBasicFont{doc, font, widths: width_map, encoding: encoding_table, default_width: None, unicode_map}
    }

    fn get_encoding(&self) -> &'a Object {
        get_obj(self.doc, self.font.get("Encoding").unwrap())
    }
    fn get_type(&self) -> String {
        get_name(self.doc, self.font, "Type")
    }
    fn get_basefont(&self) -> String {
        get_name(self.doc, self.font, "BaseFont")
    }
    fn get_subtype(&self) -> String {
        get_name(self.doc, self.font, "Subtype")
    }
    fn get_widths(&self) -> Option<&Vec<Object>> {
        maybe_get_obj(self.doc, self.font, "Widths").map(|widths| widths.as_array().expect("Widths should be an array"))
    }
    /* For type1: This entry is obsolescent and its use is no longer recommended. (See
     * implementation note 42 in Appendix H.) */
    fn get_name(&self) -> Option<String> {
        maybe_get_name(self.doc, self.font, "Name")
    }

    fn get_descriptor(&self) -> Option<PdfFontDescriptor> {
        maybe_get_obj(self.doc, self.font, "FontDescriptor").and_then(|desc| desc.as_dict()).map(|desc| PdfFontDescriptor{desc: desc, doc: self.doc})
    }
}

type CharCode = u32;

struct PdfFontIter<'a>
{
    i: Iter<'a, u8>,
    font: &'a PdfFont,
}

impl<'a> Iterator for PdfFontIter<'a> {
    type Item = CharCode;
    fn next(&mut self) -> Option<CharCode> {
        self.font.next_char(&mut self.i)
    }
}

trait PdfFont : Debug {
    fn get_width(&self, id: i64) -> f64;
    fn next_char(&self, iter: &mut Iter<u8>) -> Option<CharCode>;
    fn decode_char(&self, char: CharCode) -> String;

        /*fn char_codes<'a>(&'a self, chars: &'a [u8]) -> PdfFontIter {
            let p = self;
            PdfFontIter{i: chars.iter(), font: p as &PdfFont}
        }*/

}

impl<'a> PdfFont + 'a {
    fn char_codes(&'a self, chars: &'a [u8]) -> PdfFontIter {
        PdfFontIter{i: chars.iter(), font: self}
    }
    fn decode(&self, chars: &[u8]) -> String {
        let strings = self.char_codes(chars).map(|x| self.decode_char(x)).collect::<Vec<_>>();
        strings.join("")
    }

}


impl<'a> PdfFont for PdfBasicFont<'a> {
    fn get_width(&self, id: i64) -> f64 {
        let width = self.widths.get(&id);
        if let Some(width) = width {
            return *width;
        } else {
            println!("missing width for {} falling back to default_width", id);
            return self.default_width.unwrap();
        }
    }
    /*fn decode(&self, chars: &[u8]) -> String {
        let encoding = self.encoding.as_ref().map(|x| &x[..]).unwrap_or(&PDFDocEncoding);
        to_utf8(encoding, chars)
    }*/

    fn next_char(&self, iter: &mut Iter<u8>) -> Option<CharCode> {
        iter.next().map(|x| *x as CharCode)
    }
    fn decode_char(&self, char: CharCode) -> String {
        let slice = [char as u8];
        if let Some(ref unicode_map) = self.unicode_map {
            let s = unicode_map.get(&char);
            let s = match s {
                None => { panic!("missing char {:?} in map {:?}", char, unicode_map)}
                Some(s) => { *s }
            };
            let s = [s as u16];
            let s = String::from_utf16(&s).unwrap();
            return s
        }
        let encoding = self.encoding.as_ref().map(|x| &x[..]).unwrap_or(&PDFDocEncoding);
        //println!("char_code {:?} {:?}", char, self.encoding);
        let s = to_utf8(encoding, &slice);
        s
    }
}



impl<'a> fmt::Debug for PdfBasicFont<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.font.fmt(f)
    }
}

struct PdfCIDFont<'a> {
    font: &'a Dictionary,
    doc: &'a Document,
    encoding: Option<Vec<u16>>,
    to_unicode: Option<HashMap<u32, u32>>,
    widths: HashMap<i64, f64>, // should probably just use i32 here
    default_width: Option<f64>, // only used for CID fonts and we should probably brake out the different font types
}

fn get_unicode_map<'a>(doc: &'a Document, font: &'a Dictionary) -> Option<HashMap<u32, u32>> {
    let to_unicode = maybe_get_obj(doc, font, "ToUnicode");
    //println!("ToUnicode: {:?}", to_unicode);
    let mut unicode_map = None;
    match to_unicode {
        Some(&Object::Stream(ref stream)) => {
            let contents = get_contents(stream);
            unicode_map = Some(adobe_cmap_parser::get_unicode_map(&contents).unwrap());
            println!("Stream: {:?} {:?}", unicode_map, contents);
        }
        None => { }
        _ => { panic!("unsupported cmap")}
    }
    unicode_map
}


impl<'a> PdfCIDFont<'a> {
    fn new(doc: &'a Document, font: &'a Dictionary) -> PdfCIDFont<'a> {
        let base_name = get_name(doc, font, "BaseFont");
        let descendants = maybe_get_array(doc, font, "DescendantFonts").expect("Descendant fonts required");
        let ciddict = maybe_deref(doc, &descendants[0]).as_dict().expect("should be CID dict");
        let encoding = maybe_get_obj(doc, font, "Encoding").expect("Encoding required in type0 fonts");
        println!("base_name {} {:?}", base_name, font);

        match encoding {
            &Object::Name(ref name) => {
                let name = pdf_to_utf8(name);
                assert!(name == "Identity-H");

                println!("encoding {:?}", name);
            }
            _ => { panic!("unsupported encoding")}
        }

        // Sometimes a Type0 font might refer to the same underlying data as regular font. In this case we may be able to extract some encoding
        // data.
        // We should also look inside the truetype data to see if there's a cmap table. It will help us convert as well.
        // This won't work if the cmap has been subsetted. A better approach might be to hash glyph contents and use that against
        // a global library of glyph hashes
        let unicode_map = get_unicode_map(doc, font);

        println!("descendents {:?} {:?}", descendants, ciddict);

        let font_dict = maybe_get_obj(doc, ciddict, "FontDescriptor").expect("required");
        println!("{:?}", font_dict);
        let f = font_dict.as_dict().expect("must be dict");
        let default_width = maybe_get_obj(doc, ciddict, "DW").and_then(|x| x.as_i64()).unwrap_or(1000);
        let w = maybe_get_array(doc, ciddict, "W").expect("widths");
        println!("widths {:?}", w);
        let mut widths = HashMap::new();
        let mut i = 0;
        while i < w.len() {
            if let Object::Array(ref wa) = w[i+1] {
                let cid = w[i].as_i64().expect("id should be num");
                let mut j = 0;
                println!("wa: {:?} -> {:?}", cid, wa);
                for w in wa {
                    widths.insert(cid + j, as_num(w) );
                    j += 1;
                }
                i += 2;
            } else {
                let c_first = w[i].as_i64().expect("first should be num");
                let c_last = w[i].as_i64().expect("last should be num");
                let c_width = as_num(&w[i]);
                for id in c_first..c_last {
                    widths.insert(id, c_width);
                }
                i += 3;
            }
        }
        PdfCIDFont{doc, font, widths, to_unicode: unicode_map, encoding: None, default_width: Some(default_width as f64) }
    }
}

impl<'a> PdfFont for PdfCIDFont<'a> {
    fn get_width(&self, id: i64) -> f64 {
        let width = self.widths.get(&id);
        if let Some(width) = width {
            println!("GetWidth {} -> {}", id, *width);
            return *width;
        } else {
            println!("missing width for {} falling back to default_width", id);
            return self.default_width.unwrap();
        }
    }/*
    fn decode(&self, chars: &[u8]) -> String {
        self.char_codes(chars);

        //let utf16 = Vec::new();

        let encoding = self.encoding.as_ref().map(|x| &x[..]).unwrap_or(&PDFDocEncoding);
        to_utf8(encoding, chars)
    }*/

    fn next_char(&self, iter: &mut Iter<u8>) -> Option<CharCode> {
        let p = iter.next();
        if let Some(&c) = p {
            let next = *iter.next().unwrap();
            Some(((c as u32) << 8)  | next as u32)
        } else {
            None
        }
    }
    fn decode_char(&self, char: CharCode) -> String {
        let s = self.to_unicode.as_ref().unwrap()[&char];
        let s = [s as u16];
        let s = String::from_utf16(&s).unwrap();
        s
    }
}

impl<'a> fmt::Debug for PdfCIDFont<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.font.fmt(f)
    }
}



#[derive(Copy, Clone)]
struct PdfFontDescriptor<'a> {
    desc: &'a Dictionary,
    doc: &'a Document
}

impl<'a> PdfFontDescriptor<'a> {
    fn get_file(&self) -> Option<&'a Object> {
        maybe_get_obj(self.doc, self.desc, "FontFile")
    }
    fn get_name(&self) -> String {
        pdf_to_utf8(get_obj(self.doc, self.desc.get("Name").unwrap()).as_name().unwrap())
    }
}

impl<'a> fmt::Debug for PdfFontDescriptor<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.desc.fmt(f)
    }
}

fn as_num(o: &Object) -> f64 {
    match o {
        &Object::Integer(i) => { i as f64 }
        &Object::Real(f) => { f }
        _ => { panic!("not a number") }
    }
}

#[derive(Clone)]
struct TextState<'a>
{
    font: Option<Rc<PdfFont + 'a>>,
    font_size: f64,
    character_spacing: f64,
    word_spacing: f64,
    horizontal_scaling: f64,
    rise: f64,
}

// XXX: We'd ideally implement this without having to copy the uncompressed data
fn get_contents(contents: &Stream) -> Vec<u8> {
    if contents.filter().is_some() {
        contents.decompressed_content().unwrap()
    } else {
        contents.content.clone()
    }
}

#[derive(Clone)]
struct GraphicsState<'a>
{
    ctm: euclid::Transform2D<f64>,
    ts: TextState<'a>
}

fn show_text(ts: &TextState, s: &[u8], gs: &GraphicsState,
             tm: &mut euclid::Transform2D<f64>,
             tlm: &euclid::Transform2D<f64>,
             flip_ctm: &euclid::Transform2D<f64>,
             output: &mut TextOutput ) {
    let font = ts.font.as_ref().unwrap();
    //let encoding = font.encoding.as_ref().map(|x| &x[..]).unwrap_or(&PDFDocEncoding);
    println!("{:?}", font.decode(s));
    output.begin_word();

    for c in font.char_codes(s) {
        let tsm = euclid::Transform2D::row_major(ts.font_size * ts.horizontal_scaling,
                                                 0.,
                                                 0.,
                                                 ts.horizontal_scaling,
                                                 0.,
                                                 ts.rise);
        let trm = tm.pre_mul(&gs.ctm);
        let position = trm.post_mul(&flip_ctm);
        //println!("ctm: {:?} tm {:?}", gs.ctm, tm);
        println!("current pos: {:?}", position);
        // 5.9 Extraction of Text Content


        //println!("w: {}", font.widths[&(*c as i64)]);
        let w0 = font.get_width(c as i64) / 1000.;
        output.output_character(position.m31, position.m32, w0,ts.font_size, &font.decode_char(c));
        let tj = 0.;
        let ty = 0.;
        let tx = ts.horizontal_scaling * ((w0 - tj/1000.)* ts.font_size + ts.word_spacing + ts.character_spacing);
        // println!("w0: {}, tx: {}", w0, tx);
        *tm = tm.pre_mul(&euclid::Transform2D::create_translation(tx, ty));
        let trm = tm.pre_mul(&gs.ctm);
        //println!("post pos: {:?}", trm);

    }
    output.end_word();
}

#[derive(Debug)]
struct MediaBox {
    llx: f64,
    lly: f64,
    urx: f64,
    ury: f64
}

fn process_stream(doc: &Document, contents: &Stream, fonts: &Dictionary, media_box: &MediaBox, output: &mut TextOutput, page_num: u32) {
    let data = get_contents(contents);
    //println!("contents {}", pdf_to_utf8(&data));
    let content = Content::decode(&data).unwrap();
    let mut gs : GraphicsState = GraphicsState{ts: TextState { font: None, font_size: std::f64::NAN, character_spacing: 0.,
                             word_spacing: 0., horizontal_scaling: 100. / 100., rise: 0.},
    ctm: euclid::Transform2D::identity()};
    //let mut ts = &mut gs.ts;
    let mut gs_stack = Vec::new();
    let mut tm = euclid::Transform2D::identity();
    let mut tlm = euclid::Transform2D::identity();
    let mut flip_ctm = euclid::Transform2D::row_major(1., 0., 0., -1., 0., (media_box.ury - media_box.lly));
    println!("MediaBox {:?}", media_box);
    for operation in &content.operations {
        //println!("op: {:?}", operation);
        match operation.operator.as_ref() {
            "BT" => {
                tlm = euclid::Transform2D::identity();
                tm = tlm;
            }
            "ET" => {
                tlm = euclid::Transform2D::identity();
                tm = tlm;
            }
            "cm" => {
                assert!(operation.operands.len() == 6);
                let m = euclid::Transform2D::row_major(as_num(&operation.operands[0]),
                                                       as_num(&operation.operands[1]),
                                                       as_num(&operation.operands[2]),
                                                       as_num(&operation.operands[3]),
                                                       as_num(&operation.operands[4]),
                                                       as_num(&operation.operands[5]));
                gs.ctm = gs.ctm.pre_mul(&m);
                println!("matrix {:?}", gs.ctm);
            }
            "TJ" => {
                let mut ts = &gs.ts;
                match operation.operands[0] {
                    Object::Array(ref array) => {
                        for e in array {
                            match e {
                                &Object::String(ref s, _) => {
                                    show_text(&gs.ts, s, &gs, &mut tm, &tlm, &flip_ctm, output);
                                }
                                &Object::Integer(i) => {
                                    let w0 = 0.;
                                    let tj = i as f64;
                                    let ty = 0.;
                                    let tx = ts.horizontal_scaling * ((w0 - tj/1000.)* ts.font_size + ts.word_spacing + ts.character_spacing);
                                    tm = tm.pre_mul(&euclid::Transform2D::create_translation(tx, ty));
                                    println!("adjust text by: {} {:?}", i, tm);
                                }
                                &Object::Real(i) => {
                                    let w0 = 0.;
                                    let tj = i as f64;
                                    let ty = 0.;
                                    let tx = ts.horizontal_scaling * ((w0 - tj/1000.)* ts.font_size + ts.word_spacing + ts.character_spacing);
                                    tm = tm.pre_mul(&euclid::Transform2D::create_translation(tx, ty));
                                    println!("adjust text by: {} {:?}", i, tm);
                                }
                                _ => { println!("kind of {:?}", e);}
                            }
                        }
                    }
                    _ => {}
                }
            }
            "Tj" => {
                match operation.operands[0] {
                    Object::String(ref s, _) => {
                        show_text(&gs.ts, s, &gs, &mut tm, &tlm, &flip_ctm, output);
                    }
                    _ => { panic!("unexpected Tj operand {:?}", operation) }
                }
            }
            "Ts" => {
                gs.ts.rise = as_num(&operation.operands[0]);
            }
            "Tz" => {
                gs.ts.horizontal_scaling = as_num(&operation.operands[0]) / 100.;
            }
            "Tf" => {
                let font = make_font(doc, get_obj(doc, fonts.get(str::from_utf8(operation.operands[0].as_name().unwrap()).unwrap()).unwrap()).as_dict().unwrap());
                {
                    /*let file = font.get_descriptor().and_then(|desc| desc.get_file());
                    if let Some(file) = file {
                        let file_contents = filter_data(file.as_stream().unwrap());
                        let mut cursor = Cursor::new(&file_contents[..]);
                        //let f = Font::read(&mut cursor);
                        //println!("font file: {:?}", f);
                    }*/
                }
                gs.ts.font = Some(font);

                gs.ts.font_size = as_num(&operation.operands[1]);
                println!("font size: {}", gs.ts.font_size);
            }
            "Tm" => {
                assert!(operation.operands.len() == 6);
                tlm = euclid::Transform2D::row_major(as_num(&operation.operands[0]),
                                                       as_num(&operation.operands[1]),
                                                       as_num(&operation.operands[2]),
                                                       as_num(&operation.operands[3]),
                                                       as_num(&operation.operands[4]),
                                                       as_num(&operation.operands[5]));
                tm = tlm;
                println!("Tm: matrix {:?}", tm);

            }
            "Td" => {
                /* Move to the start of the next line, offset from the start of the current line by (tx , ty ).
                   tx and ty are numbers expressed in unscaled text space units.
                   More precisely, this operator performs the following assignments:
                 */
                assert!(operation.operands.len() == 2);
                let tx = as_num(&operation.operands[0]);
                let ty = as_num(&operation.operands[1]);
                println!("translation: {} {}", tx, ty);

                tlm = tlm.pre_mul(&euclid::Transform2D::create_translation(tx, ty));
                tm = tlm;
                println!("Td matrix {:?}", tm);
                output.end_line();

            }
            "q" => { gs_stack.push(gs.clone()); }
            "Q" => { gs = gs_stack.pop().unwrap(); }
            "w" | "J" | "j" | "M" | "d" | "ri" | "i" | "gs" => { println!("unknown graphics state operator {:?}", operation); }
            "m" | "l" | "c" | "v" | "y" | "h" | "re" => { println!("unknown path construction operator {:?}", operation); }
            _ => { println!("unknown operation {:?}", operation);}
        }
    }

}


trait TextOutput {
    fn begin_page(&mut self, page_num: u32, media_box: &MediaBox);
    fn end_page(&mut self);
    fn output_character(&mut self, x: f64, y: f64, width: f64, font_size: f64, char: &str);
    fn begin_word(&mut self);
    fn end_word(&mut self);
    fn end_line(&mut self);
}


struct HTMLOutput<'a>  {
    file: &'a mut File
}

impl<'a> TextOutput for HTMLOutput<'a> {
    fn begin_page(&mut self, page_num: u32, media_box: &MediaBox) {
        write!(self.file, "<!-- page {} -->", page_num);
        write!(self.file, "<div id='page{}' style='position: relative; height: {}px; width: {}px; border: 1px black solid'>", page_num, media_box.ury - media_box.lly, media_box.urx - media_box.llx);
    }
    fn end_page(&mut self) {
        write!(self.file, "</div>");
    }
    fn output_character(&mut self, x: f64, y: f64, width: f64, font_size: f64, char: &str) {
        write!(self.file, "<div style='position: absolute; left: {}px; top: {}px; font-size: {}px'>{}</div>",
               x, y, font_size, char);
    }
    fn begin_word(&mut self) {}
    fn end_word(&mut self) {}
    fn end_line(&mut self) {}
}

struct PlainTextOutput<'a>  {
    file: &'a mut File,
    last_end: f64,
    first_char: bool
}

impl<'a> PlainTextOutput<'a> {
    fn new(file: &mut File) -> PlainTextOutput {
        PlainTextOutput{file, last_end: 100000., first_char: false}
    }
}

impl<'a> TextOutput for PlainTextOutput<'a> {
    fn begin_page(&mut self, page_num: u32, media_box: &MediaBox) {
    }
    fn end_page(&mut self) {
    }
    fn output_character(&mut self, x: f64, y: f64, width: f64, font_size: f64, char: &str) {
        if self.first_char {
            if x > self.last_end + width*0.5 {
                write!(self.file, " ");
            }
        }
        write!(self.file, "{}", char);
        self.first_char = false;
        self.last_end = x + width;
    }
    fn begin_word(&mut self) {
        self.first_char = true;
    }
    fn end_word(&mut self) {}
    fn end_line(&mut self) {
        write!(self.file, "\n");
    }
}

fn main() {
    let file = env::args().nth(1).unwrap();
    println!("{}", file);
    let path = Path::new(&file);
    let filename = path.file_name().expect("expected a filename");
    let mut output_file = PathBuf::new();
    output_file.push(filename);
    output_file.set_extension("html");
    let mut output_file = File::create(output_file).expect("could not create output");
    write!(&mut output_file, "<meta charset='utf-8' /> ");
    let doc = Document::load(path).unwrap();
    println!("Version: {}", doc.version);
    if let Some(ref info) = get_info(&doc) {
        for (k, v) in *info {
            match v {
                &Object::String(ref s, StringFormat::Literal) => { println!("{}: {}", k, pdf_to_utf8(s)); }
                _ => {}
            }
        }
    }
    println!("Page count: {}", get_pages(&doc).get("Count").unwrap().as_i64().unwrap());
    println!("Pages: {:?}", get_pages(&doc));
    println!("Type: {:?}", get_pages(&doc).get("Type").and_then(|x| x.as_name()).unwrap());

    let mut html_output = PlainTextOutput::new(&mut output_file);
    extract_text(&doc, &mut html_output);
}


fn extract_text(doc: &Document, output: &mut TextOutput) {
    let pages = doc.get_pages();
    for dict in pages {
        let page_num = dict.0;
        let dict = doc.get_object(dict.1).unwrap().as_dict().unwrap();
        println!("page {} {:?}", page_num, dict);
        let resources = maybe_deref(&doc, dict.get("Resources").unwrap()).as_dict().unwrap();
        println!("resources {:?}", resources);
        let font = resources.get("Font").unwrap().as_dict().unwrap();
        println!("font {:?}", font);

        // pdfium searches up the page tree for MediaBoxes as needed
        let media_box = maybe_get_array(&doc, dict, "MediaBox")
            .expect("Should have been a MediaBox")
            .iter()
            .map(|x| as_num(x)).collect::<Vec<_>>();
        let media_box = MediaBox{llx: media_box[0], lly: media_box[1], urx: media_box[2], ury: media_box[3]};

        output.begin_page(page_num, &media_box);
        // Contents can point to either an array of references or a single reference
        match dict.get("Contents") {
            Some(&Object::Reference(ref id)) => {
                match doc.get_object(*id).unwrap() {
                    &Object::Stream(ref contents) => {
                        process_stream(&doc, contents, font, &media_box, output, page_num);
                    }

                    _ => {}
                }
            }
            Some(&Object::Array(ref arr)) => {
                for id in arr {
                    let id = id.as_reference().unwrap();
                    match doc.get_object(id).unwrap() {
                        &Object::Stream(ref contents) => {
                            process_stream(&doc, contents, font, &media_box, output, page_num);
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
        output.end_page();
    }
}
