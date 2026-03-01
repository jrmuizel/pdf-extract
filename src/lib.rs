use adobe_cmap_parser::{ByteMapping, CodeRange, CIDRange};
use encoding_rs::UTF_16BE;
pub use hayro_syntax::*;
use hayro_syntax::Pdf;
use hayro_syntax::object::{Object, Dict, Stream, Name, Number, Array};
use hayro_syntax::object::Rect;
use hayro_syntax::content::UntypedIter;
use hayro_syntax::page::Resources;
use euclid::*;
use std::fmt::{Debug, Formatter};
use std::sync::Arc;
extern crate encoding_rs;
extern crate euclid;
extern crate adobe_cmap_parser;
extern crate type1_encoding_parser;
extern crate unicode_normalization;
use euclid::vec2;
use unicode_normalization::UnicodeNormalization;
use std::fmt;
use std::str;
use std::fs::File;
use std::slice::Iter;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::rc::Rc;
use std::marker::PhantomData;
use std::result::Result;
use log::{warn, error, debug};
mod core_fonts;
mod glyphnames;
mod zapfglyphnames;
mod encodings;

pub struct Space;
pub type Transform = Transform2D<f64, Space, Space>;

#[derive(Debug)]
pub enum OutputError
{
    FormatError(std::fmt::Error),
    IoError(std::io::Error),
    PdfError(LoadPdfError)
}

impl std::fmt::Display for OutputError
{
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            OutputError::FormatError(e) => write!(f, "Formating error: {}", e),
            OutputError::IoError(e) => write!(f, "IO error: {}", e),
            OutputError::PdfError(e) => write!(f, "PDF error: {:?}", e)
        }
    }
}

impl std::error::Error for OutputError{}

impl From<std::fmt::Error> for OutputError {
    fn from(e: std::fmt::Error) -> Self {
        OutputError::FormatError(e)
    }
}

impl From<std::io::Error> for OutputError {
    fn from(e: std::io::Error) -> Self {
        OutputError::IoError(e)
    }
}

impl From<LoadPdfError> for OutputError {
    fn from(e: LoadPdfError) -> Self {
        OutputError::PdfError(e)
    }
}

macro_rules! dlog {
    ($($e:expr),*) => { {$(let _ = $e;)*} }
    //($($t:tt)*) => { println!($($t)*) }
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
        return UTF_16BE.decode_without_bom_handling_and_without_replacement(&s[2..]).unwrap().to_string()
    } else {
        let r : Vec<u8> = s.iter().map(|x| *x).flat_map(|x| {
               let k = PDFDocEncoding[x as usize];
               vec![(k>>8) as u8, k as u8].into_iter()}).collect();
        return UTF_16BE.decode_without_bom_handling_and_without_replacement(&r).unwrap().to_string()
    }
}

fn to_utf8(encoding: &[u16], s: &[u8]) -> String {
    if s.len() > 2 && s[0] == 0xfe && s[1] == 0xff {
        return UTF_16BE.decode_without_bom_handling_and_without_replacement(&s[2..]).unwrap().to_string()
    } else {
        let r : Vec<u8> = s.iter().map(|x| *x).flat_map(|x| {
            let k = encoding[x as usize];
            vec![(k>>8) as u8, k as u8].into_iter()}).collect();
        return UTF_16BE.decode_without_bom_handling_and_without_replacement(&r).unwrap().to_string()
    }
}

fn as_num(o: &Object) -> f64 {
    match o {
        Object::Number(n) => n.as_f64(),
        _ => { panic!("not a number") }
    }
}

// Helper to get a Name from a dict and convert to String
fn get_name_string(dict: &Dict, key: &[u8]) -> String {
    let name: Name = dict.get(key).unwrap_or_else(|| panic!("missing key {:?}", std::str::from_utf8(key)));
    pdf_to_utf8(&name)
}

#[allow(dead_code)]
fn maybe_get_name_string(dict: &Dict, key: &[u8]) -> Option<String> {
    dict.get::<Name>(key).map(|n| pdf_to_utf8(&n))
}

fn maybe_get_name<'a>(dict: &Dict<'a>, key: &[u8]) -> Option<Name<'a>> {
    dict.get::<Name>(key)
}

fn maybe_get_array<'a>(dict: &Dict<'a>, key: &[u8]) -> Option<Array<'a>> {
    dict.get::<Array>(key)
}

// XXX: We'd ideally implement this without having to copy the uncompressed data
fn get_stream_contents(stream: &Stream) -> Vec<u8> {
    stream.decoded().unwrap_or_else(|_| stream.raw_data().into_owned())
}

#[derive(Clone)]
struct PdfSimpleFont<'a> {
    font: Dict<'a>,
    encoding: Option<Vec<u16>>,
    unicode_map: Option<HashMap<u32, String>>,
    widths: HashMap<CharCode, f64>,
    missing_width: f64,
}

#[derive(Clone)]
struct PdfType3Font<'a> {
    font: Dict<'a>,
    encoding: Option<Vec<u16>>,
    unicode_map: Option<HashMap<CharCode, String>>,
    widths: HashMap<CharCode, f64>,
}


fn make_font<'a>(font: &Dict<'a>) -> Rc<dyn PdfFont + 'a> {
    let subtype = get_name_string(font, b"Subtype");
    dlog!("MakeFont({})", subtype);
    if subtype == "Type0" {
        Rc::new(PdfCIDFont::new(font))
    } else if subtype == "Type3" {
        Rc::new(PdfType3Font::new(font))
    } else {
        Rc::new(PdfSimpleFont::new(font))
    }
}

fn is_core_font(name: &str) -> bool {
    match name {
        "Courier-Bold" |
        "Courier-BoldOblique" |
        "Courier-Oblique" |
        "Courier" |
        "Helvetica-Bold" |
        "Helvetica-BoldOblique" |
        "Helvetica-Oblique" |
        "Helvetica" |
        "Symbol" |
        "Times-Bold" |
        "Times-BoldItalic" |
        "Times-Italic" |
        "Times-Roman" |
        "ZapfDingbats" => true,
        _ => false,
    }
}

fn encoding_to_unicode_table(name: &[u8]) -> Vec<u16> {
    let encoding = match &name[..] {
        b"MacRomanEncoding" => encodings::MAC_ROMAN_ENCODING,
        b"MacExpertEncoding" => encodings::MAC_EXPERT_ENCODING,
        b"WinAnsiEncoding" => encodings::WIN_ANSI_ENCODING,
        _ => panic!("unexpected encoding {:?}", pdf_to_utf8(name))
    };
    let encoding_table = encoding.iter()
        .map(|x| if let &Some(x) = x { glyphnames::name_to_unicode(x).unwrap() } else { 0 })
        .collect();
    encoding_table
}

impl<'a> PdfSimpleFont<'a> {
    fn new(font: &Dict<'a>) -> PdfSimpleFont<'a> {
        let base_name = get_name_string(font, b"BaseFont");
        let subtype = get_name_string(font, b"Subtype");

        let encoding: Option<Object> = font.get::<Object>(&b"Encoding"[..]);
        dlog!("base_name {} {} enc:{:?} {:?}", base_name, subtype, encoding, font);
        let descriptor: Option<Dict> = font.get::<Dict>(&b"FontDescriptor"[..]);
        let mut type1_encoding = None;
        let mut unicode_map = None;
        if let Some(ref descriptor) = descriptor {
            dlog!("descriptor {:?}", descriptor);
            if subtype == "Type1" {
                let file: Option<Stream> = descriptor.get::<Stream>(&b"FontFile"[..]);
                match file {
                    Some(ref s) => {
                        let s = get_stream_contents(s);
                        type1_encoding = Some(type1_encoding_parser::get_encoding_map(&s).expect("encoding"));
                    }
                    None => { dlog!("font file: None") }
                }
            } else if subtype == "TrueType" {
                let file: Option<Stream> = descriptor.get::<Stream>(&b"FontFile2"[..]);
                match file {
                    Some(ref s) => {
                        let _s = get_stream_contents(s);
                    }
                    None => { dlog!("font file: None") }
                }
            }

            let font_file3: Option<Stream> = descriptor.get::<Stream>(&b"FontFile3"[..]);
            match font_file3 {
                Some(ref s) => {
                    let subtype_name = get_name_string(s.dict(), b"Subtype");
                    dlog!("font file {}", subtype_name);
                    let contents = get_stream_contents(s);
                    if subtype_name == "Type1C" {
                        let table = cff_parser::Table::parse(&contents).unwrap();

                        let enc = table.encoding.get_code_to_sid_table(&table.charset);

                        let mapping: HashMap<u32, String> = enc.into_iter().filter_map(|(cid, sid)| {
                            let name = cff_parser::string_by_id(&table, sid).unwrap();
                            if name == ".notdef" {
                                return None;
                            }
                            let unicode = glyphnames::name_to_unicode(&name).or_else(|| {
                                zapfglyphnames::zapfdigbats_names_to_unicode(name)
                            });
                            if unicode.is_none() {
                                warn!("Couldn't find unicode for {}", name);
                                return None;
                            }
                            let s = String::from_utf16(&[unicode.unwrap()]).unwrap();
                            Some((cid as u32, s))
                        }).collect();
                        unicode_map = Some(mapping);
                    }
                }
                None => {}
            }

            let charset: Option<hayro_syntax::object::String> = descriptor.get(&b"CharSet"[..]);
            let _charset = match charset {
                Some(ref s) => { Some(pdf_to_utf8(&s.get())) }
                None => { None }
            };
        }

        let mut unicode_map = match unicode_map {
            Some(mut unicode_map) => {
                unicode_map.extend(get_unicode_map(font).unwrap_or(HashMap::new()));
                Some(unicode_map)
            }
            None => {
                get_unicode_map(font)
            }
        };

        let mut encoding_table = None;
        match encoding {
            Some(Object::Name(ref encoding_name)) => {
                dlog!("encoding {:?}", pdf_to_utf8(encoding_name));
                encoding_table = Some(encoding_to_unicode_table(encoding_name));
            }
            Some(Object::Dict(ref encoding_dict)) => {
                let mut table = if let Some(base_encoding) = maybe_get_name(encoding_dict, b"BaseEncoding") {
                    dlog!("BaseEncoding {:?}", base_encoding);
                    encoding_to_unicode_table(&base_encoding)
                } else {
                    Vec::from(PDFDocEncoding)
                };
                let differences = maybe_get_array(encoding_dict, b"Differences");
                if let Some(differences) = differences {
                    dlog!("Differences");
                    let mut code: i64 = 0;
                    for o in differences.iter::<Object>() {
                        match o {
                            Object::Number(n) => { code = n.as_i64(); },
                            Object::Name(ref n) => {
                                let name = pdf_to_utf8(&n);
                                let unicode = glyphnames::name_to_unicode(&name);
                                if let Some(unicode) = unicode{
                                    table[code as usize] = unicode;
                                    if let Some(ref mut unicode_map) = unicode_map {
                                        let be = [unicode];
                                        match unicode_map.entry(code as u32) {
                                            Entry::Vacant(v) => { v.insert(String::from_utf16(&be).unwrap()); }
                                            Entry::Occupied(e) => {
                                                if e.get() != &String::from_utf16(&be).unwrap() {
                                                    let normal_match  = e.get().nfkc().eq(String::from_utf16(&be).unwrap().nfkc());
                                                    if !normal_match {
                                                        warn!("Unicode mismatch {} {} {:?} {:?} {:?}", normal_match, name, e.get(), String::from_utf16(&be), be);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                } else {
                                    match unicode_map {
                                        Some(ref mut unicode_map) if base_name.contains("FontAwesome") => {
                                            match unicode_map.entry(code as u32) {
                                                Entry::Vacant(v) => { v.insert("".to_owned()); }
                                                Entry::Occupied(_e) => {
                                                    panic!("unexpected entry in unicode map")
                                                }
                                            }
                                        }
                                        _ => {
                                            warn!("unknown glyph name '{}' for font {}", name, base_name);
                                        }
                                    }
                                }
                                dlog!("{} = {} ({:?})", code, name, unicode);
                                if let Some(ref unicode_map) = unicode_map {
                                    dlog!("{} {:?}", code, unicode_map.get(&(code as u32)));
                                }
                                code += 1;
                            }
                            _ => { panic!("wrong type {:?}", o); }
                        }
                    }
                }
                let _name = encoding_dict.get::<Name>(&b"Type"[..]).map(|n| pdf_to_utf8(&n));
                dlog!("name: {:?}", _name);

                encoding_table = Some(table);
            }
            None => {
                if let Some(type1_encoding) = type1_encoding {
                    let mut table = Vec::from(PDFDocEncoding);
                    dlog!("type1encoding");
                    for (code, name) in type1_encoding {
                        let unicode = glyphnames::name_to_unicode(&pdf_to_utf8(&name));
                        if let Some(unicode) = unicode {
                            table[code as usize] = unicode;
                        } else {
                            dlog!("unknown character {}", pdf_to_utf8(&name));
                        }
                    }
                    encoding_table = Some(table)
                } else if subtype == "TrueType" {
                    encoding_table = Some(encodings::WIN_ANSI_ENCODING.iter()
                        .map(|x| if let &Some(x) = x { glyphnames::name_to_unicode(x).unwrap() } else { 0 })
                        .collect());
                }
            }
            _ => { panic!() }
        }

        let mut width_map = HashMap::new();

        let first_char: Option<i64> = font.get::<Number>(&b"FirstChar"[..]).map(|n| n.as_i64());
        let last_char: Option<i64> = font.get::<Number>(&b"LastChar"[..]).map(|n| n.as_i64());
        let widths: Option<Vec<f64>> = font.get::<Array>(&b"Widths"[..]).map(|a| a.iter::<Number>().map(|n| n.as_f64()).collect());

        if let (Some(first_char), Some(last_char), Some(widths)) = (first_char, last_char, widths) {
            let mut i: i64 = 0;
            dlog!("first_char {:?}, last_char: {:?}, widths: {} {:?}", first_char, last_char, widths.len(), widths);

            for w in widths {
                width_map.insert((first_char + i) as CharCode, w);
                i += 1;
            }
            assert_eq!(first_char + i - 1, last_char);
        } else {
            let name = if is_core_font(&base_name) {
                &base_name
            } else {
                warn!("no widths and not core font {:?}", base_name);
                "Helvetica"
            };
            for font_metrics in core_fonts::metrics().iter() {
                if font_metrics.0 == base_name {
                    if let Some(ref encoding) = encoding_table {
                        dlog!("has encoding");
                        for w in font_metrics.2 {
                            let c = glyphnames::name_to_unicode(w.2).unwrap();
                            for i in 0..encoding.len() {
                                if encoding[i] == c {
                                    width_map.insert(i as CharCode, w.1 as f64);
                                }
                            }
                        }
                    } else {
                        let mut table = vec![0; 256];
                        for w in font_metrics.2 {
                            dlog!("{} {}", w.0, w.2);
                            if w.0 != -1 {
                                table[w.0 as usize] = if base_name == "ZapfDingbats" {
                                    zapfglyphnames::zapfdigbats_names_to_unicode(w.2).unwrap_or_else(|| panic!("bad name {:?}", w))
                                } else {
                                    glyphnames::name_to_unicode(w.2).unwrap()
                                }
                            }
                        }

                        let encoding = &table[..];
                        for w in font_metrics.2 {
                            width_map.insert(w.0 as CharCode, w.1 as f64);
                        }
                        encoding_table = Some(encoding.to_vec());
                    }
                }
            }
        }

        let missing_width = font.get::<Number>(&b"MissingWidth"[..]).map(|n| n.as_f64()).unwrap_or(0.);
        PdfSimpleFont {font: font.clone(), widths: width_map, encoding: encoding_table, missing_width, unicode_map}
    }

    #[allow(dead_code)]
    fn get_type(&self) -> String {
        get_name_string(&self.font, b"Type")
    }
    #[allow(dead_code)]
    fn get_basefont(&self) -> String {
        get_name_string(&self.font, b"BaseFont")
    }
    #[allow(dead_code)]
    fn get_subtype(&self) -> String {
        get_name_string(&self.font, b"Subtype")
    }
    #[allow(dead_code)]
    fn get_widths(&self) -> Option<Vec<f64>> {
        self.font.get::<Array>(&b"Widths"[..]).map(|a| a.iter::<Number>().map(|n| n.as_f64()).collect())
    }
    #[allow(dead_code)]
    fn get_name(&self) -> Option<String> {
        maybe_get_name_string(&self.font, b"Name")
    }

    #[allow(dead_code)]
    fn get_descriptor(&self) -> Option<PdfFontDescriptor<'a>> {
        self.font.get::<Dict>(&b"FontDescriptor"[..]).map(|desc| PdfFontDescriptor{desc})
    }
}



impl<'a> PdfType3Font<'a> {
    fn new(font: &Dict<'a>) -> PdfType3Font<'a> {

        let unicode_map = get_unicode_map(font);
        let encoding: Option<Object> = font.get::<Object>(&b"Encoding"[..]);

        let encoding_table;
        match encoding {
            Some(Object::Name(ref encoding_name)) => {
                dlog!("encoding {:?}", pdf_to_utf8(encoding_name));
                encoding_table = Some(encoding_to_unicode_table(encoding_name));
            }
            Some(Object::Dict(ref encoding_dict)) => {
                let mut table = if let Some(base_encoding) = maybe_get_name(encoding_dict, b"BaseEncoding") {
                    dlog!("BaseEncoding {:?}", base_encoding);
                    encoding_to_unicode_table(&base_encoding)
                } else {
                    Vec::from(PDFDocEncoding)
                };
                let differences = maybe_get_array(encoding_dict, b"Differences");
                if let Some(differences) = differences {
                    dlog!("Differences");
                    let mut code: i64 = 0;
                    for o in differences.iter::<Object>() {
                        match o {
                            Object::Number(n) => { code = n.as_i64(); },
                            Object::Name(ref n) => {
                                let name = pdf_to_utf8(&n);
                                let unicode = glyphnames::name_to_unicode(&name);
                                if let Some(unicode) = unicode{
                                    table[code as usize] = unicode;
                                }
                                dlog!("{} = {} ({:?})", code, name, unicode);
                                if let Some(ref unicode_map) = unicode_map {
                                    dlog!("{} {:?}", code, unicode_map.get(&(code as u32)));
                                }
                                code += 1;
                            }
                            _ => { panic!("wrong type"); }
                        }
                    }
                }
                let _name = encoding_dict.get::<Name>(&b"Type"[..]).map(|n| pdf_to_utf8(&n));
                dlog!("name: {:?}", _name);

                encoding_table = Some(table);
            }
            _ => { panic!() }
        }

        let first_char: i64 = font.get::<Number>(&b"FirstChar"[..]).expect("FirstChar").as_i64();
        let last_char: i64 = font.get::<Number>(&b"LastChar"[..]).expect("LastChar").as_i64();
        let widths: Vec<f64> = font.get::<Array>(&b"Widths"[..]).expect("Widths").iter::<Number>().map(|n| n.as_f64()).collect();

        let mut width_map = HashMap::new();

        let mut i = 0;
        dlog!("first_char {:?}, last_char: {:?}, widths: {} {:?}", first_char, last_char, widths.len(), widths);

        for w in widths {
            width_map.insert((first_char + i) as CharCode, w);
            i += 1;
        }
        assert_eq!(first_char + i - 1, last_char);
        PdfType3Font {font: font.clone(), widths: width_map, encoding: encoding_table, unicode_map}
    }
}

type CharCode = u32;

struct PdfFontIter<'a>
{
    i: Iter<'a, u8>,
    font: &'a dyn PdfFont,
}

impl<'a> Iterator for PdfFontIter<'a> {
    type Item = (CharCode, u8);
    fn next(&mut self) -> Option<(CharCode, u8)> {
        self.font.next_char(&mut self.i)
    }
}

trait PdfFont : Debug {
    fn get_width(&self, id: CharCode) -> f64;
    fn next_char(&self, iter: &mut Iter<u8>) -> Option<(CharCode, u8)>;
    fn decode_char(&self, char: CharCode) -> String;
}

impl<'a> dyn PdfFont + 'a {
    fn char_codes(&'a self, chars: &'a [u8]) -> PdfFontIter {
        PdfFontIter{i: chars.iter(), font: self}
    }
    fn decode(&self, chars: &[u8]) -> String {
        let strings = self.char_codes(chars).map(|x| self.decode_char(x.0)).collect::<Vec<_>>();
        strings.join("")
    }
}


impl<'a> PdfFont for PdfSimpleFont<'a> {
    fn get_width(&self, id: CharCode) -> f64 {
        let width = self.widths.get(&id);
        if let Some(width) = width {
            return *width;
        } else {
            let mut widths = self.widths.iter().collect::<Vec<_>>();
            widths.sort_by_key(|x| x.0);
            dlog!("missing width for {} len(widths) = {}, {:?} falling back to missing_width {:?}", id, self.widths.len(), widths, self.font);
            return self.missing_width;
        }
    }

    fn next_char(&self, iter: &mut Iter<u8>) -> Option<(CharCode, u8)> {
        iter.next().map(|x| (*x as CharCode, 1))
    }
    fn decode_char(&self, char: CharCode) -> String {
        let slice = [char as u8];
        if let Some(ref unicode_map) = self.unicode_map {
            let s = unicode_map.get(&char);
            let s = match s {
                None => {
                    debug!("missing char {:?} in unicode map {:?} for {:?}", char, unicode_map, self.font);
                    let encoding = self.encoding.as_ref().map(|x| &x[..]).expect("missing unicode map and encoding");
                    let s = to_utf8(encoding, &slice);
                    debug!("falling back to encoding {} -> {:?}", char, s);
                    s
                }
                Some(s) => { s.clone() }
            };
            return s
        }
        let encoding = self.encoding.as_ref().map(|x| &x[..]).unwrap_or(&PDFDocEncoding);
        let s = to_utf8(encoding, &slice);
        s
    }
}



impl<'a> fmt::Debug for PdfSimpleFont<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.font.fmt(f)
    }
}

impl<'a> PdfFont for PdfType3Font<'a> {
    fn get_width(&self, id: CharCode) -> f64 {
        let width = self.widths.get(&id);
        if let Some(width) = width {
            return *width;
        } else {
            panic!("missing width for {} {:?}", id, self.font);
        }
    }

    fn next_char(&self, iter: &mut Iter<u8>) -> Option<(CharCode, u8)> {
        iter.next().map(|x| (*x as CharCode, 1))
    }
    fn decode_char(&self, char: CharCode) -> String {
        let slice = [char as u8];
        if let Some(ref unicode_map) = self.unicode_map {
            let s = unicode_map.get(&char);
            let s = match s {
                None => {
                    debug!("missing char {:?} in unicode map {:?} for {:?}", char, unicode_map, self.font);
                    let encoding = self.encoding.as_ref().map(|x| &x[..]).expect("missing unicode map and encoding");
                    let s = to_utf8(encoding, &slice);
                    debug!("falling back to encoding {} -> {:?}", char, s);
                    s
                }
                Some(s) => { s.clone() }
            };
            return s
        }
        let encoding = self.encoding.as_ref().map(|x| &x[..]).unwrap_or(&PDFDocEncoding);
        let s = to_utf8(encoding, &slice);
        s
    }
}



impl<'a> fmt::Debug for PdfType3Font<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.font.fmt(f)
    }
}

struct PdfCIDFont<'a> {
    font: Dict<'a>,
    #[allow(dead_code)]
    encoding: ByteMapping,
    to_unicode: Option<HashMap<u32, String>>,
    widths: HashMap<CharCode, f64>,
    default_width: Option<f64>,
}

fn get_unicode_map<'a>(font: &Dict<'a>) -> Option<HashMap<u32, String>> {
    let to_unicode: Option<Object> = font.get::<Object>(&b"ToUnicode"[..]);
    dlog!("ToUnicode: {:?}", to_unicode);
    let mut unicode_map = None;
    match to_unicode {
        Some(Object::Stream(ref stream)) => {
            let contents = get_stream_contents(stream);
            dlog!("Stream: {}", String::from_utf8(contents.clone()).unwrap());

            let cmap = adobe_cmap_parser::get_unicode_map(&contents).unwrap();
            let mut unicode = HashMap::new();
            for (&k, v) in cmap.iter() {
                let mut be: Vec<u16> = Vec::new();
                let mut i = 0;
                assert!(v.len() % 2 == 0);
                while i < v.len() {
                    be.push(((v[i] as u16) << 8) | v[i+1] as u16);
                    i += 2;
                }
                match &be[..] {
                    [0xd800 ..= 0xdfff] => {
                        continue;
                    }
                    _ => {}
                }
                let s = String::from_utf16(&be).unwrap();

                unicode.insert(k, s);
            }
            unicode_map = Some(unicode);

            dlog!("map: {:?}", unicode_map);
        }
        None => { }
        Some(Object::Name(ref name)) => {
            let name = pdf_to_utf8(name);
            if name != "Identity-H" {
                todo!("unsupported ToUnicode name: {:?}", name);
            }
        }
        _ => { panic!("unsupported cmap {:?}", to_unicode)}
    }
    unicode_map
}


impl<'a> PdfCIDFont<'a> {
    fn new(font: &Dict<'a>) -> PdfCIDFont<'a> {
        let base_name = get_name_string(font, b"BaseFont");
        let descendants: Array = font.get::<Array>(&b"DescendantFonts"[..]).expect("Descendant fonts required");
        let ciddict: Dict = descendants.iter::<Dict>().next().expect("should have CID dict");
        let encoding_obj: Object = font.get::<Object>(&b"Encoding"[..]).expect("Encoding required in type0 fonts");
        dlog!("base_name {} {:?}", base_name, font);

        let encoding = match encoding_obj {
            Object::Name(ref name) => {
                let name_str = pdf_to_utf8(name);
                dlog!("encoding {:?}", name_str);
                if name_str == "Identity-H" || name_str == "Identity-V" {
                    ByteMapping { codespace: vec![CodeRange{width: 2, start: 0, end: 0xffff }], cid: vec![CIDRange{ src_code_lo: 0, src_code_hi: 0xffff, dst_CID_lo: 0 }]}
                } else {
                    panic!("unsupported encoding {}", name_str);
                }
            }
            Object::Stream(ref stream) => {
                let contents = get_stream_contents(stream);
                dlog!("Stream: {}", String::from_utf8(contents.clone()).unwrap());
                adobe_cmap_parser::get_byte_mapping(&contents).unwrap()
            }
            _ => { panic!("unsupported encoding {:?}", encoding_obj)}
        };

        let unicode_map = get_unicode_map(font);

        dlog!("descendents {:?}", ciddict);

        let _font_dict: Dict = ciddict.get::<Dict>(&b"FontDescriptor"[..]).expect("required");
        dlog!("{:?}", _font_dict);
        let default_width = ciddict.get::<Number>(&b"DW"[..]).map(|n| n.as_i64()).unwrap_or(1000);
        let w: Option<Array> = ciddict.get::<Array>(&b"W"[..]);
        dlog!("widths {:?}", w);
        let mut widths = HashMap::new();
        if let Some(w) = w {
            let w_objs: Vec<Object> = w.iter::<Object>().collect();
            let mut i = 0;
            while i < w_objs.len() {
                // Check if second element is an array
                if i + 1 < w_objs.len() {
                    if let Object::Array(ref wa) = w_objs[i+1] {
                        let cid = match &w_objs[i] {
                            Object::Number(n) => n.as_i64(),
                            _ => panic!("id should be num"),
                        };
                        let mut j = 0;
                        for w_item in wa.iter::<Number>() {
                            widths.insert((cid + j) as CharCode, w_item.as_f64());
                            j += 1;
                        }
                        i += 2;
                        continue;
                    }
                }
                // Otherwise it's c_first c_last width
                if i + 2 < w_objs.len() {
                    let c_first = match &w_objs[i] {
                        Object::Number(n) => n.as_i64(),
                        _ => panic!("first should be num"),
                    };
                    let c_last = match &w_objs[i+1] {
                        Object::Number(n) => n.as_i64(),
                        _ => panic!("last should be num"),
                    };
                    let c_width = match &w_objs[i+2] {
                        Object::Number(n) => n.as_f64(),
                        _ => panic!("width should be num"),
                    };
                    for id in c_first..=c_last {
                        widths.insert(id as CharCode, c_width);
                    }
                    i += 3;
                } else {
                    break;
                }
            }
        }
        PdfCIDFont{font: font.clone(), widths, to_unicode: unicode_map, encoding, default_width: Some(default_width as f64) }
    }
}

impl<'a> PdfFont for PdfCIDFont<'a> {
    fn get_width(&self, id: CharCode) -> f64 {
        let width = self.widths.get(&id);
        if let Some(width) = width {
            dlog!("GetWidth {} -> {}", id, *width);
            return *width;
        } else {
            dlog!("missing width for {} falling back to default_width", id);
            return self.default_width.unwrap();
        }
    }

    fn next_char(&self, iter: &mut Iter<u8>) -> Option<(CharCode, u8)> {
        let mut c = *iter.next()? as u32;
        let mut code = None;
        'outer: for width in 1..=4 {
            for range in &self.encoding.codespace {
                if c as u32 >= range.start && c as u32 <= range.end && range.width == width {
                    code = Some((c as u32, width));
                    break 'outer;
                }
            }
            let next = *iter.next()?;
            c = ((c as u32) << 8) | next as u32;
        }
        let code = code?;
        for range in &self.encoding.cid {
            if code.0 >= range.src_code_lo && code.0 <= range.src_code_hi {
                return Some((code.0 + range.dst_CID_lo, code.1 as u8));
            }
        }
        None
    }
    fn decode_char(&self, char: CharCode) -> String {
        let s = self.to_unicode.as_ref().and_then(|x| x.get(&char));
        if let Some(s) = s {
            s.clone()
        } else {
            dlog!("Unknown character {:?} in {:?} {:?}", char, self.font, self.to_unicode);
            "".to_string()
        }
    }
}

impl<'a> fmt::Debug for PdfCIDFont<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.font.fmt(f)
    }
}



#[derive(Clone)]
struct PdfFontDescriptor<'a> {
    desc: Dict<'a>,
}

impl<'a> PdfFontDescriptor<'a> {
    #[allow(dead_code)]
    fn get_file(&self) -> Option<Stream<'a>> {
        self.desc.get::<Stream>(&b"FontFile"[..])
    }
}

impl<'a> fmt::Debug for PdfFontDescriptor<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.desc.fmt(f)
    }
}

#[derive(Clone, Debug)]
struct Type0Func {
    domain: Vec<f64>,
    range: Vec<f64>,
    contents: Vec<u8>,
    size: Vec<i64>,
    bits_per_sample: i64,
    encode: Vec<f64>,
    decode: Vec<f64>,
}

#[allow(dead_code)]
fn interpolate(x: f64, x_min: f64, _x_max: f64, y_min: f64, y_max: f64) -> f64 {
    let divisor = x - x_min;
    if divisor != 0. {
        y_min + (x - x_min) * ((y_max - y_min) / divisor)
    } else {
        y_min
    }
}

impl Type0Func {
    #[allow(dead_code)]
    fn eval(&self, _input: &[f64], _output: &mut [f64]) {
        let _n_inputs = self.domain.len() / 2;
        let _n_ouputs = self.range.len() / 2;
    }
}

#[derive(Clone, Debug)]
struct Type2Func {
    c0: Option<Vec<f64>>,
    c1: Option<Vec<f64>>,
    n: f64,
}

#[derive(Clone, Debug)]
enum Function {
    Type0(Type0Func),
    Type2(Type2Func),
    #[allow(dead_code)]
    Type3,
    #[allow(dead_code)]
    Type4(Vec<u8>)
}

fn get_dict_from_obj<'a>(obj: &Object<'a>) -> Option<Dict<'a>> {
    match obj {
        Object::Dict(d) => Some(d.clone()),
        Object::Stream(s) => Some(s.dict().clone()),
        _ => None,
    }
}

impl Function {
    fn new(obj: &Object) -> Function {
        let dict = get_dict_from_obj(obj).expect("function must be dict or stream");
        let function_type: i64 = dict.get::<Number>(&b"FunctionType"[..]).expect("FunctionType").as_i64();
        let f = match function_type {
            0 => {
                let stream = match obj {
                    Object::Stream(ref stream) => stream,
                    _ => panic!()
                };
                let range: Vec<f64> = dict.get::<Array>(&b"Range"[..]).expect("Range").iter::<Number>().map(|n| n.as_f64()).collect();
                let domain: Vec<f64> = dict.get::<Array>(&b"Domain"[..]).expect("Domain").iter::<Number>().map(|n| n.as_f64()).collect();
                let contents = get_stream_contents(stream);
                let size: Vec<i64> = dict.get::<Array>(&b"Size"[..]).expect("Size").iter::<Number>().map(|n| n.as_i64()).collect();
                let bits_per_sample = dict.get::<Number>(&b"BitsPerSample"[..]).expect("BitsPerSample").as_i64();

                let encode: Option<Vec<f64>> = dict.get::<Array>(&b"Encode"[..]).map(|a| a.iter::<Number>().map(|n| n.as_f64()).collect());
                let encode = encode.unwrap_or_else(|| {
                    let mut default = Vec::new();
                    for i in &size {
                        default.extend([0., (i - 1) as f64].iter());
                    }
                    default
                });
                let decode: Option<Vec<f64>> = dict.get::<Array>(&b"Decode"[..]).map(|a| a.iter::<Number>().map(|n| n.as_f64()).collect());
                let decode = decode.unwrap_or_else(|| range.clone());

                Function::Type0(Type0Func { domain, range, size, contents, bits_per_sample, encode, decode })
            }
            2 => {
                let c0: Option<Vec<f64>> = dict.get::<Array>(&b"C0"[..]).map(|a| a.iter::<Number>().map(|n| n.as_f64()).collect());
                let c1: Option<Vec<f64>> = dict.get::<Array>(&b"C1"[..]).map(|a| a.iter::<Number>().map(|n| n.as_f64()).collect());
                let n = dict.get::<Number>(&b"N"[..]).expect("N").as_f64();
                Function::Type2(Type2Func { c0, c1, n})
            }
            3 => {
                Function::Type3
            }
            4 => {
                let contents = match obj {
                    Object::Stream(ref stream) => {
                        let contents = get_stream_contents(stream);
                        warn!("unhandled type-4 function");
                        warn!("Stream: {}", String::from_utf8(contents.clone()).unwrap());
                        contents
                    }
                    _ => { panic!("type 4 functions should be streams") }
                };
                Function::Type4(contents)
            }
            _ => { panic!("unhandled function type {}", function_type) }
        };
        f
    }
}

#[derive(Clone)]
struct TextState<'a>
{
    font: Option<Rc<dyn PdfFont + 'a>>,
    font_size: f64,
    character_spacing: f64,
    word_spacing: f64,
    horizontal_scaling: f64,
    leading: f64,
    rise: f64,
    tm: Transform,
}

#[derive(Clone)]
struct GraphicsState<'a>
{
    ctm: Transform,
    ts: TextState<'a>,
    smask: Option<Dict<'a>>,
    fill_colorspace: ColorSpace,
    fill_color: Vec<f64>,
    stroke_colorspace: ColorSpace,
    stroke_color: Vec<f64>,
    line_width: f64,
}

fn show_text(gs: &mut GraphicsState, s: &[u8],
             _tlm: &Transform,
             _flip_ctm: &Transform,
             output: &mut dyn OutputDev) -> Result<(), OutputError> {
    let ts = &mut gs.ts;
    let font = ts.font.as_ref().unwrap();
    dlog!("{:?}", font.decode(s));
    dlog!("{:?}", font.decode(s).as_bytes());
    dlog!("{:?}", s);
    output.begin_word()?;

    for (c, length) in font.char_codes(s) {
        let tsm = Transform2D::row_major(ts.horizontal_scaling,
                                                 0.,
                                                 0.,
                                                 1.0,
                                                 0.,
                                                 ts.rise);
        let trm = tsm.post_transform(&ts.tm.post_transform(&gs.ctm));
        let w0 = font.get_width(c) / 1000.;

        let mut spacing = ts.character_spacing;
        let is_space = c == 32 && length == 1;
        if is_space { spacing += ts.word_spacing }

        output.output_character(&trm, w0, spacing, ts.font_size, &font.decode_char(c))?;
        let tj = 0.;
        let ty = 0.;
        let tx = ts.horizontal_scaling * ((w0 - tj/1000.)* ts.font_size + spacing);
        dlog!("horizontal {} adjust {} {} {} {}", ts.horizontal_scaling, tx, w0, ts.font_size, spacing);
        ts.tm = ts.tm.pre_transform(&Transform2D::create_translation(tx, ty));
        let _trm = ts.tm.pre_transform(&gs.ctm);
    }
    output.end_word()?;
    Ok(())
}

#[derive(Debug, Clone, Copy)]
pub struct MediaBox {
    pub llx: f64,
    pub lly: f64,
    pub urx: f64,
    pub ury: f64
}

fn apply_state<'a>(gs: &mut GraphicsState<'a>, state: &Dict<'a>) {
    for (k, v) in state.entries() {
        let v = match v {
            hayro_syntax::object::MaybeRef::Ref(_) => continue,
            hayro_syntax::object::MaybeRef::NotRef(obj) => obj,
        };
        match &*k {
            b"SMask" => { match v {
                Object::Name(ref name) => {
                    if &**name == b"None" {
                        gs.smask = None;
                    } else {
                        panic!("unexpected smask name")
                    }
                }
                Object::Dict(ref dict) => {
                    gs.smask = Some(dict.clone());
                }
                _ => { panic!("unexpected smask type {:?}", v) }
            }}
            b"Type" => { match v {
                Object::Name(ref name) => {
                    assert_eq!(&**name, b"ExtGState")
                }
                _ => { panic!("unexpected type") }
            }}
            _ => { dlog!("unapplied state: {:?} {:?}", k, v); }
        }
    }
}

#[derive(Debug)]
pub enum PathOp {
    MoveTo(f64, f64),
    LineTo(f64, f64),
    CurveTo(f64, f64, f64, f64, f64, f64),
    Rect(f64, f64, f64, f64),
    Close,
}

#[derive(Debug)]
pub struct Path {
    pub ops: Vec<PathOp>
}

impl Path {
    fn new() -> Path {
        Path { ops: Vec::new() }
    }
    fn current_point(&self) -> (f64, f64) {
        match self.ops.last().unwrap() {
            &PathOp::MoveTo(x, y) => { (x, y) }
            &PathOp::LineTo(x, y) => { (x, y) }
            &PathOp::CurveTo(_, _, _, _, x, y) => { (x, y) }
            _ => { panic!() }
        }
    }
}

#[derive(Clone, Debug)]
pub struct CalGray {
    white_point: [f64; 3],
    black_point: Option<[f64; 3]>,
    gamma: Option<f64>,
}

#[derive(Clone, Debug)]
pub struct CalRGB {
    white_point: [f64; 3],
    black_point: Option<[f64; 3]>,
    gamma: Option<[f64; 3]>,
    matrix: Option<Vec<f64>>
}

#[derive(Clone, Debug)]
pub struct Lab {
    white_point: [f64; 3],
    black_point: Option<[f64; 3]>,
    range: Option<[f64; 4]>,
}

#[derive(Clone, Debug)]
pub enum AlternateColorSpace {
    DeviceGray,
    DeviceRGB,
    DeviceCMYK,
    CalRGB(CalRGB),
    CalGray(CalGray),
    Lab(Lab),
    ICCBased(Vec<u8>)
}

#[derive(Clone)]
pub struct Separation {
    name: String,
    alternate_space: AlternateColorSpace,
    tint_transform: Box<Function>,
}

#[derive(Clone)]
pub enum ColorSpace {
    DeviceGray,
    DeviceRGB,
    DeviceCMYK,
    DeviceN,
    Pattern,
    CalRGB(CalRGB),
    CalGray(CalGray),
    Lab(Lab),
    Separation(Separation),
    ICCBased(Vec<u8>)
}

fn get_array_nums(dict: &Dict, key: &[u8]) -> Option<Vec<f64>> {
    dict.get::<Array>(key).map(|a| a.iter::<Number>().map(|n| n.as_f64()).collect())
}

fn get_array_3(dict: &Dict, key: &[u8]) -> [f64; 3] {
    let v = get_array_nums(dict, key).expect(std::str::from_utf8(key).unwrap());
    [v[0], v[1], v[2]]
}

fn maybe_get_array_3(dict: &Dict, key: &[u8]) -> Option<[f64; 3]> {
    get_array_nums(dict, key).map(|v| [v[0], v[1], v[2]])
}

fn maybe_get_array_4(dict: &Dict, key: &[u8]) -> Option<[f64; 4]> {
    get_array_nums(dict, key).map(|v| [v[0], v[1], v[2], v[3]])
}

fn maybe_get_f64(dict: &Dict, key: &[u8]) -> Option<f64> {
    dict.get::<Number>(key).map(|n| n.as_f64())
}

fn make_alternate_colorspace(obj: &Object) -> AlternateColorSpace {
    match obj {
        Object::Name(ref name) => {
            match &**name {
                b"DeviceGray" => AlternateColorSpace::DeviceGray,
                b"DeviceRGB" => AlternateColorSpace::DeviceRGB,
                b"DeviceCMYK" => AlternateColorSpace::DeviceCMYK,
                _ => panic!("unexpected color space name")
            }
        }
        Object::Array(ref cs) => {
            let cs_items: Vec<Object> = cs.iter::<Object>().collect();
            let cs_name = pdf_to_utf8(cs_items[0].clone().into_name().expect("first arg must be a name").as_ref());
            match cs_name.as_ref() {
                "ICCBased" => {
                    let stream = cs_items[1].clone().into_stream().unwrap();
                    AlternateColorSpace::ICCBased(get_stream_contents(&stream))
                }
                "CalGray" => {
                    let dict = cs_items[1].clone().into_dict().expect("second arg must be a dict");
                    AlternateColorSpace::CalGray(CalGray {
                        white_point: get_array_3(&dict, b"WhitePoint"),
                        black_point: maybe_get_array_3(&dict, b"BackPoint"),
                        gamma: maybe_get_f64(&dict, b"Gamma"),
                    })
                }
                "CalRGB" => {
                    let dict = cs_items[1].clone().into_dict().expect("second arg must be a dict");
                    AlternateColorSpace::CalRGB(CalRGB {
                        white_point: get_array_3(&dict, b"WhitePoint"),
                        black_point: maybe_get_array_3(&dict, b"BackPoint"),
                        gamma: maybe_get_array_3(&dict, b"Gamma"),
                        matrix: get_array_nums(&dict, b"Matrix"),
                    })
                }
                "Lab" => {
                    let dict = cs_items[1].clone().into_dict().expect("second arg must be a dict");
                    AlternateColorSpace::Lab(Lab {
                        white_point: get_array_3(&dict, b"WhitePoint"),
                        black_point: maybe_get_array_3(&dict, b"BackPoint"),
                        range: maybe_get_array_4(&dict, b"Range"),
                    })
                }
                _ => panic!("Unexpected color space name")
            }
        }
        _ => panic!("Alternate space should be name or array {:?}", obj)
    }
}

fn make_colorspace_from_obj(obj: &Object) -> ColorSpace {
    match obj {
        Object::Name(ref name) => {
            match &**name {
                b"DeviceRGB" => ColorSpace::DeviceRGB,
                b"DeviceGray" => ColorSpace::DeviceGray,
                b"DeviceCMYK" => ColorSpace::DeviceCMYK,
                _ => panic!("unexpected colorspace name")
            }
        }
        Object::Array(ref cs) => {
            let cs_items: Vec<Object> = cs.iter::<Object>().collect();
            let cs_name = pdf_to_utf8(cs_items[0].clone().into_name().expect("first arg must be a name").as_ref());
            match cs_name.as_ref() {
                "Separation" => {
                    let name = pdf_to_utf8(cs_items[1].clone().into_name().expect("second arg must be a name").as_ref());
                    let alternate_space = make_alternate_colorspace(&cs_items[2]);
                    let tint_transform = Box::new(Function::new(&cs_items[3]));
                    dlog!("{:?} {:?} {:?}", name, alternate_space, tint_transform);
                    ColorSpace::Separation(Separation{ name, alternate_space, tint_transform})
                }
                "ICCBased" => {
                    let stream = cs_items[1].clone().into_stream().unwrap();
                    dlog!("ICCBased {:?}", stream);
                    ColorSpace::ICCBased(get_stream_contents(&stream))
                }
                "CalGray" => {
                    let dict = cs_items[1].clone().into_dict().expect("second arg must be a dict");
                    ColorSpace::CalGray(CalGray {
                        white_point: get_array_3(&dict, b"WhitePoint"),
                        black_point: maybe_get_array_3(&dict, b"BackPoint"),
                        gamma: maybe_get_f64(&dict, b"Gamma"),
                    })
                }
                "CalRGB" => {
                    let dict = cs_items[1].clone().into_dict().expect("second arg must be a dict");
                    ColorSpace::CalRGB(CalRGB {
                        white_point: get_array_3(&dict, b"WhitePoint"),
                        black_point: maybe_get_array_3(&dict, b"BackPoint"),
                        gamma: maybe_get_array_3(&dict, b"Gamma"),
                        matrix: get_array_nums(&dict, b"Matrix"),
                    })
                }
                "Lab" => {
                    let dict = cs_items[1].clone().into_dict().expect("second arg must be a dict");
                    ColorSpace::Lab(Lab {
                        white_point: get_array_3(&dict, b"WhitePoint"),
                        black_point: maybe_get_array_3(&dict, b"BackPoint"),
                        range: maybe_get_array_4(&dict, b"Range"),
                    })
                }
                "Pattern" => {
                    ColorSpace::Pattern
                },
                "DeviceGray" => ColorSpace::DeviceGray,
                "DeviceRGB" => ColorSpace::DeviceRGB,
                "DeviceCMYK" => ColorSpace::DeviceCMYK,
                "DeviceN" => ColorSpace::DeviceN,
                _ => {
                    panic!("color_space {:?} {:?}", cs_name, cs_items)
                }
            }
        }
        _ => panic!("unexpected colorspace object")
    }
}

fn make_colorspace<'a>(name: &[u8], resources: &Resources<'a>) -> ColorSpace {
    match name {
        b"DeviceGray" => ColorSpace::DeviceGray,
        b"DeviceRGB" => ColorSpace::DeviceRGB,
        b"DeviceCMYK" => ColorSpace::DeviceCMYK,
        b"Pattern" => ColorSpace::Pattern,
        _ => {
            let cs_name = Name::new(name);
            let cs: Object = resources.get_color_space(cs_name).unwrap_or_else(|| panic!("missing colorspace {:?}", std::str::from_utf8(name)));
            make_colorspace_from_obj(&cs)
        }
    }
}

struct Processor<'a> {
    _none: PhantomData<&'a ()>
}

impl<'a> Processor<'a> {
    fn new() -> Processor<'a> {
        Processor { _none: PhantomData }
    }

    fn process_stream(&mut self, content: &[u8], resources: &Resources<'a>, media_box: &MediaBox, output: &mut dyn OutputDev, page_num: u32) -> Result<(), OutputError> {
        let mut font_table: HashMap<Vec<u8>, Rc<dyn PdfFont + 'a>> = HashMap::new();
        let mut gs: GraphicsState = GraphicsState {
            ts: TextState {
                font: None,
                font_size: std::f64::NAN,
                character_spacing: 0.,
                word_spacing: 0.,
                horizontal_scaling: 100. / 100.,
                leading: 0.,
                rise: 0.,
                tm: Transform2D::identity(),
            },
            fill_color: Vec::new(),
            fill_colorspace: ColorSpace::DeviceGray,
            stroke_color: Vec::new(),
            stroke_colorspace: ColorSpace::DeviceGray,
            line_width: 1.,
            ctm: Transform2D::identity(),
            smask: None
        };
        let mut gs_stack = Vec::new();
        let mut mc_stack: Vec<Vec<u8>> = Vec::new();
        let mut tlm = Transform2D::identity();
        let mut path = Path::new();
        let flip_ctm = Transform2D::row_major(1., 0., 0., -1., 0., media_box.ury - media_box.lly);
        dlog!("MediaBox {:?}", media_box);
        for instruction in UntypedIter::new(content) {
            let op: Vec<u8> = instruction.operator.to_vec();
            let operands: Vec<Object> = instruction.operands().collect();

            match &op[..] {
                b"BT" => {
                    tlm = Transform2D::identity();
                    gs.ts.tm = tlm;
                }
                b"ET" => {
                    tlm = Transform2D::identity();
                    gs.ts.tm = tlm;
                }
                b"cm" => {
                    assert!(operands.len() == 6);
                    let m = Transform2D::row_major(as_num(&operands[0]),
                                                   as_num(&operands[1]),
                                                   as_num(&operands[2]),
                                                   as_num(&operands[3]),
                                                   as_num(&operands[4]),
                                                   as_num(&operands[5]));
                    gs.ctm = gs.ctm.pre_transform(&m);
                    dlog!("matrix {:?}", gs.ctm);
                }
                b"CS" => {
                    let name = operands[0].clone().into_name().unwrap();
                    gs.stroke_colorspace = make_colorspace(&name, resources);
                }
                b"cs" => {
                    let name = operands[0].clone().into_name().unwrap();
                    gs.fill_colorspace = make_colorspace(&name, resources);
                }
                b"SC" | b"SCN" => {
                    gs.stroke_color = match gs.stroke_colorspace {
                        ColorSpace::Pattern => { dlog!("unhandled pattern color"); Vec::new() }
                        _ => { operands.iter().map(|x| as_num(x)).collect() }
                    };
                }
                b"sc" | b"scn" => {
                    gs.fill_color = match gs.fill_colorspace {
                        ColorSpace::Pattern => { dlog!("unhandled pattern color"); Vec::new() }
                        _ => { operands.iter().map(|x| as_num(x)).collect() }
                    };
                }
                b"G" | b"g" | b"RG" | b"rg" | b"K" | b"k" => {
                    dlog!("unhandled color operation {:?} {:?}", op, operands);
                }
                b"TJ" => {
                    match operands.into_iter().next() {
                        Some(Object::Array(ref array)) => {
                            for e in array.iter::<Object>() {
                                match e {
                                    Object::String(ref s) => {
                                        show_text(&mut gs, &s.get(), &tlm, &flip_ctm, output)?;
                                    }
                                    Object::Number(n) => {
                                        let ts = &mut gs.ts;
                                        let w0 = 0.;
                                        let tj = n.as_f64();
                                        let ty = 0.;
                                        let tx = ts.horizontal_scaling * ((w0 - tj / 1000.) * ts.font_size);
                                        ts.tm = ts.tm.pre_transform(&Transform2D::create_translation(tx, ty));
                                        dlog!("adjust text by: {} {:?}", tj, ts.tm);
                                    }
                                    _ => { dlog!("kind of {:?}", e); }
                                }
                            }
                        }
                        _ => {}
                    }
                }
                b"Tj" => {
                    match &operands[0] {
                        Object::String(ref s) => {
                            show_text(&mut gs, &s.get(), &tlm, &flip_ctm, output)?;
                        }
                        _ => { panic!("unexpected Tj operand {:?}", operands) }
                    }
                }
                b"Tc" => {
                    gs.ts.character_spacing = as_num(&operands[0]);
                }
                b"Tw" => {
                    gs.ts.word_spacing = as_num(&operands[0]);
                }
                b"Tz" => {
                    gs.ts.horizontal_scaling = as_num(&operands[0]) / 100.;
                }
                b"TL" => {
                    gs.ts.leading = as_num(&operands[0]);
                }
                b"Tf" => {
                    let name = operands[0].clone().into_name().unwrap();
                    let font_name = Name::new(&name);
                    let font = font_table.entry(name.to_vec()).or_insert_with(|| {
                        let font_dict: Dict = resources.get_font(font_name).expect("font not found");
                        make_font(&font_dict)
                    }).clone();
                    gs.ts.font = Some(font);
                    gs.ts.font_size = as_num(&operands[1]);
                    dlog!("font {:?} size: {} {:?}", name, gs.ts.font_size, operands);
                }
                b"Ts" => {
                    gs.ts.rise = as_num(&operands[0]);
                }
                b"Tm" => {
                    assert!(operands.len() == 6);
                    tlm = Transform2D::row_major(as_num(&operands[0]),
                                                 as_num(&operands[1]),
                                                 as_num(&operands[2]),
                                                 as_num(&operands[3]),
                                                 as_num(&operands[4]),
                                                 as_num(&operands[5]));
                    gs.ts.tm = tlm;
                    dlog!("Tm: matrix {:?}", gs.ts.tm);
                    output.end_line()?;
                }
                b"Td" => {
                    assert!(operands.len() == 2);
                    let tx = as_num(&operands[0]);
                    let ty = as_num(&operands[1]);
                    dlog!("translation: {} {}", tx, ty);

                    tlm = tlm.pre_transform(&Transform2D::create_translation(tx, ty));
                    gs.ts.tm = tlm;
                    dlog!("Td matrix {:?}", gs.ts.tm);
                    output.end_line()?;
                }

                b"TD" => {
                    assert!(operands.len() == 2);
                    let tx = as_num(&operands[0]);
                    let ty = as_num(&operands[1]);
                    dlog!("translation: {} {}", tx, ty);
                    gs.ts.leading = -ty;

                    tlm = tlm.pre_transform(&Transform2D::create_translation(tx, ty));
                    gs.ts.tm = tlm;
                    dlog!("TD matrix {:?}", gs.ts.tm);
                    output.end_line()?;
                }

                b"T*" => {
                    let tx = 0.0;
                    let ty = -gs.ts.leading;

                    tlm = tlm.pre_transform(&Transform2D::create_translation(tx, ty));
                    gs.ts.tm = tlm;
                    dlog!("T* matrix {:?}", gs.ts.tm);
                    output.end_line()?;
                }
                b"q" => { gs_stack.push(gs.clone()); }
                b"Q" => {
                    let s = gs_stack.pop();
                    if let Some(s) = s {
                        gs = s;
                    } else {
                        warn!("No state to pop");
                    }
                }
                b"gs" => {
                    let name = operands[0].clone().into_name().unwrap();
                    let gs_name = Name::new(&name);
                    let state: Dict = resources.get_ext_g_state(gs_name).expect("ExtGState not found");
                    apply_state(&mut gs, &state);
                }
                b"i" => { dlog!("unhandled graphics state flattness operator {:?}", operands); }
                b"w" => { gs.line_width = as_num(&operands[0]); }
                b"J" | b"j" | b"M" | b"d" | b"ri" => { dlog!("unknown graphics state operator {:?} {:?}", op, operands); }
                b"m" => { path.ops.push(PathOp::MoveTo(as_num(&operands[0]), as_num(&operands[1]))) }
                b"l" => { path.ops.push(PathOp::LineTo(as_num(&operands[0]), as_num(&operands[1]))) }
                b"c" => {
                    path.ops.push(PathOp::CurveTo(
                        as_num(&operands[0]),
                        as_num(&operands[1]),
                        as_num(&operands[2]),
                        as_num(&operands[3]),
                        as_num(&operands[4]),
                        as_num(&operands[5])))
                }
                b"v" => {
                    let (x, y) = path.current_point();
                    path.ops.push(PathOp::CurveTo(
                        x,
                        y,
                        as_num(&operands[0]),
                        as_num(&operands[1]),
                        as_num(&operands[2]),
                        as_num(&operands[3])))
                }
                b"y" => {
                    path.ops.push(PathOp::CurveTo(
                        as_num(&operands[0]),
                        as_num(&operands[1]),
                        as_num(&operands[2]),
                        as_num(&operands[3]),
                        as_num(&operands[2]),
                        as_num(&operands[3])))
                }
                b"h" => { path.ops.push(PathOp::Close) }
                b"re" => {
                    path.ops.push(PathOp::Rect(as_num(&operands[0]),
                                               as_num(&operands[1]),
                                               as_num(&operands[2]),
                                               as_num(&operands[3])))
                }
                b"s" | b"f*" | b"B" | b"B*" | b"b" => {
                    dlog!("unhandled path op {:?} {:?}", op, operands);
                }
                b"S" => {
                    output.stroke(&gs.ctm, &gs.stroke_colorspace, &gs.stroke_color, &path)?;
                    path.ops.clear();
                }
                b"F" | b"f" => {
                    output.fill(&gs.ctm, &gs.fill_colorspace, &gs.fill_color, &path)?;
                    path.ops.clear();
                }
                b"W" | b"w*" => { dlog!("unhandled clipping operation {:?} {:?}", op, operands); }
                b"n" => {
                    dlog!("discard {:?}", path);
                    path.ops.clear();
                }
                b"BMC" | b"BDC" => {
                    mc_stack.push(op.to_vec());
                }
                b"EMC" => {
                    mc_stack.pop();
                }
                b"Do" => {
                    let name = operands[0].clone().into_name().unwrap();
                    let xobj_name = Name::new(&name);
                    let xf: Stream = resources.get_x_object(xobj_name).expect("XObject not found");
                    let xf_dict = xf.dict();
                    match xf_dict.get::<Name>(&b"Subtype"[..]).as_deref() {
                        Some(b"Form") => {
                            // Build sub-resources: check if the xobject has its own Resources
                            let sub_resources_dict: Option<Dict> = xf_dict.get::<Dict>(&b"Resources"[..]);
                            let sub_resources = if let Some(res_dict) = sub_resources_dict {
                                Resources::from_parent(res_dict, resources.clone())
                            } else {
                                resources.clone()
                            };
                            let contents = get_stream_contents(&xf);
                            self.process_stream(&contents, &sub_resources, &media_box, output, page_num)?;
                        }
                        _ => {
                            // As of PDF 1.7, there are three Subtypes: "/Form", "/Image", "/PS".
                            // Only "/Form" contains nested PDF instructions,
                            // while "/Image" contains raw binary data, and "/PS" is deprecated.
                            // For most PDFs, passing image data to process_stream only produces warnings,
                            // but if the image data happen to contain a byte that can be decoded as an operator,
                            // the program may crash.
                        }
                    }
                }
                _ => { dlog!("unknown operation {:?} {:?}", op, operands); }

            }
        }
        Ok(())
    }
}


pub trait OutputDev {
    fn begin_page(&mut self, page_num: u32, media_box: &MediaBox, art_box: Option<(f64, f64, f64, f64)>)-> Result<(), OutputError>;
    fn end_page(&mut self)-> Result<(), OutputError>;
    fn output_character(&mut self, trm: &Transform, width: f64, spacing: f64, font_size: f64, char: &str) -> Result<(), OutputError>;
    fn begin_word(&mut self)-> Result<(), OutputError>;
    fn end_word(&mut self)-> Result<(), OutputError>;
    fn end_line(&mut self)-> Result<(), OutputError>;
    fn stroke(&mut self, _ctm: &Transform, _colorspace: &ColorSpace, _color: &[f64], _path: &Path)-> Result<(), OutputError> {Ok(())}
    fn fill(&mut self, _ctm: &Transform, _colorspace: &ColorSpace, _color: &[f64], _path: &Path)-> Result<(), OutputError> {Ok(())}
}


pub struct HTMLOutput<'a>  {
    file: &'a mut dyn std::io::Write,
    flip_ctm: Transform,
    last_ctm: Transform,
    buf_ctm: Transform,
    buf_font_size: f64,
    buf: String
}

fn insert_nbsp(input: &str) -> String {
    let mut result = String::new();
    let mut word_end = false;
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        if c == ' ' {
            if !word_end || chars.peek().filter(|x| **x != ' ').is_none() {
                result += "&nbsp;";
            } else {
                result += " ";
            }
            word_end = false;
        } else {
            word_end = true;
            result.push(c);
        }
    }
    result
}

impl<'a> HTMLOutput<'a> {
    pub fn new(file: &mut dyn std::io::Write) -> HTMLOutput {
        HTMLOutput {
            file,
            flip_ctm: Transform2D::identity(),
            last_ctm: Transform2D::identity(),
            buf_ctm: Transform2D::identity(),
            buf: String::new(),
            buf_font_size: 0.,
        }
    }
    fn flush_string(&mut self) -> Result<(), OutputError>{
        if self.buf.len() != 0 {

            let position = self.buf_ctm.post_transform(&self.flip_ctm);
            let transformed_font_size_vec = self.buf_ctm.transform_vector(vec2(self.buf_font_size, self.buf_font_size));
            let transformed_font_size = (transformed_font_size_vec.x * transformed_font_size_vec.y).sqrt();
            let (x, y) = (position.m31, position.m32);
            warn!("flush {} {:?}", self.buf, (x,y));

            write!(self.file, "<div style='position: absolute; left: {}px; top: {}px; font-size: {}px'>{}</div>\n",
                   x, y, transformed_font_size, insert_nbsp(&self.buf))?;
        }
        Ok(())
    }
}

type ArtBox = (f64, f64, f64, f64);

impl<'a> OutputDev for HTMLOutput<'a> {
    fn begin_page(&mut self, page_num: u32, media_box: &MediaBox, _: Option<ArtBox>) -> Result<(), OutputError> {
        write!(self.file, "<meta charset='utf-8' /> ")?;
        write!(self.file, "<!-- page {} -->", page_num)?;
        write!(self.file, "<div id='page{}' style='position: relative; height: {}px; width: {}px; border: 1px black solid'>", page_num, media_box.ury - media_box.lly, media_box.urx - media_box.llx)?;
        self.flip_ctm = Transform::row_major(1., 0., 0., -1., 0., media_box.ury - media_box.lly);
        Ok(())
    }
    fn end_page(&mut self) -> Result<(), OutputError> {
        self.flush_string()?;
        self.buf = String::new();
        self.last_ctm = Transform::identity();
        write!(self.file, "</div>")?;
        Ok(())
    }
    fn output_character(&mut self, trm: &Transform, width: f64, spacing: f64, font_size: f64, char: &str) -> Result<(), OutputError>{
        if trm.approx_eq(&self.last_ctm) {
            let position = trm.post_transform(&self.flip_ctm);
            let (x, y) = (position.m31, position.m32);

            warn!("accum {} {:?}", char, (x,y));
            self.buf += char;
        } else {
            warn!("flush {} {:?} {:?} {} {} {}", char, trm, self.last_ctm, width, font_size, spacing);
            self.flush_string()?;
            self.buf = char.to_owned();
            self.buf_font_size = font_size;
            self.buf_ctm = *trm;
        }
        let position = trm.post_transform(&self.flip_ctm);
        let transformed_font_size_vec = trm.transform_vector(vec2(font_size, font_size));
        let transformed_font_size = (transformed_font_size_vec.x * transformed_font_size_vec.y).sqrt();
        let (x, y) = (position.m31, position.m32);
        write!(self.file, "<div style='position: absolute; color: red; left: {}px; top: {}px; font-size: {}px'>{}</div>",
               x, y, transformed_font_size, char)?;
        self.last_ctm = trm.pre_transform(&Transform2D::create_translation(width * font_size + spacing, 0.));

        Ok(())
    }
    fn begin_word(&mut self) -> Result<(), OutputError> {Ok(())}
    fn end_word(&mut self) -> Result<(), OutputError> {Ok(())}
    fn end_line(&mut self) -> Result<(), OutputError> {Ok(())}
}

pub struct SVGOutput<'a>  {
    file: &'a mut dyn std::io::Write
}
impl<'a> SVGOutput<'a> {
    pub fn new(file: &mut dyn std::io::Write) -> SVGOutput {
        SVGOutput{file}
    }
}

impl<'a> OutputDev for SVGOutput<'a> {
    fn begin_page(&mut self, _page_num: u32, media_box: &MediaBox, art_box: Option<(f64, f64, f64, f64)>) -> Result<(), OutputError> {
        let ver = 1.1;
        write!(self.file, "<?xml version=\"1.0\" encoding=\"UTF-8\" ?>\n")?;
        if ver == 1.1 {
            write!(self.file, r#"<!DOCTYPE svg PUBLIC "-//W3C//DTD SVG 1.1//EN" "http://www.w3.org/Graphics/SVG/1.1/DTD/svg11.dtd">"#)?;
        } else {
            write!(self.file, r#"<!DOCTYPE svg PUBLIC "-//W3C//DTD SVG 1.0//EN" "http://www.w3.org/TR/2001/REC-SVG-20010904/DTD/svg10.dtd">"#)?;
        }
        if let Some(art_box) = art_box {
            let width = art_box.2 - art_box.0;
            let height = art_box.3 - art_box.1;
            let y = media_box.ury - art_box.1 - height;
            write!(self.file, "<svg width=\"{}\" height=\"{}\" xmlns=\"http://www.w3.org/2000/svg\" version=\"{}\" viewBox='{} {} {} {}'>", width, height, ver, art_box.0, y, width, height)?;
        } else {
            let width = media_box.urx - media_box.llx;
            let height = media_box.ury - media_box.lly;
            write!(self.file, "<svg width=\"{}\" height=\"{}\" xmlns=\"http://www.w3.org/2000/svg\" version=\"{}\" viewBox='{} {} {} {}'>", width, height, ver, media_box.llx, media_box.lly, width, height)?;
        }
        write!(self.file, "\n")?;
        type Mat = Transform;

        let ctm = Mat::create_scale(1., -1.).post_translate(vec2(0., media_box.ury));
        write!(self.file, "<g transform='matrix({}, {}, {}, {}, {}, {})'>\n",
               ctm.m11,
               ctm.m12,
               ctm.m21,
               ctm.m22,
               ctm.m31,
               ctm.m32,
        )?;
        Ok(())
    }
    fn end_page(&mut self) -> Result<(), OutputError>{
        write!(self.file, "</g>\n")?;
        write!(self.file, "</svg>")?;
        Ok(())
    }
    fn output_character(&mut self, _trm: &Transform, _width: f64, _spacing: f64, _font_size: f64, _char: &str) -> Result<(), OutputError>{
        Ok(())
    }
    fn begin_word(&mut self) -> Result<(), OutputError> {Ok(())}
    fn end_word(&mut self) -> Result<(), OutputError> {Ok(())}
    fn end_line(&mut self) -> Result<(), OutputError> {Ok(())}
    fn fill(&mut self, ctm: &Transform, _colorspace: &ColorSpace, _color: &[f64], path: &Path) -> Result<(), OutputError>{
        write!(self.file, "<g transform='matrix({}, {}, {}, {}, {}, {})'>",
               ctm.m11,
               ctm.m12,
               ctm.m21,
               ctm.m22,
               ctm.m31,
               ctm.m32,
        )?;

        let mut d = Vec::new();
        for op in &path.ops {
            match op {
                &PathOp::MoveTo(x, y) => { d.push(format!("M{} {}", x, y))}
                &PathOp::LineTo(x, y) => { d.push(format!("L{} {}", x, y))},
                &PathOp::CurveTo(x1, y1, x2, y2, x, y) => { d.push(format!("C{} {} {} {} {} {}", x1, y1, x2, y2, x, y))},
                &PathOp::Close => { d.push(format!("Z"))},
                &PathOp::Rect(x, y, width, height) => {
                    d.push(format!("M{} {}", x, y));
                    d.push(format!("L{} {}", x + width, y));
                    d.push(format!("L{} {}", x + width, y + height));
                    d.push(format!("L{} {}", x, y + height));
                    d.push(format!("Z"));
                }

            }
        }
        write!(self.file, "<path d='{}' />", d.join(" "))?;
        write!(self.file, "</g>")?;
        write!(self.file, "\n")?;
        Ok(())
    }
}

pub trait ConvertToFmt {
    type Writer: std::fmt::Write;
    fn convert(self) -> Self::Writer;
}

impl<'a> ConvertToFmt for &'a mut String {
    type Writer = &'a mut String;
    fn convert(self) -> Self::Writer {
        self
    }
}

pub struct WriteAdapter<W> {
    f: W,
}

impl<W: std::io::Write> std::fmt::Write for WriteAdapter<W> {
    fn write_str(&mut self, s: &str) -> Result<(), fmt::Error> {
        self.f.write_all(s.as_bytes()).map_err(|_| fmt::Error)
    }
}

impl<'a> ConvertToFmt for &'a mut dyn std::io::Write {
    type Writer = WriteAdapter<Self>;
    fn convert(self) -> Self::Writer {
        WriteAdapter { f: self }
    }
}

impl<'a> ConvertToFmt for &'a mut File {
    type Writer = WriteAdapter<Self>;
    fn convert(self) -> Self::Writer {
        WriteAdapter { f: self }
    }
}

pub struct PlainTextOutput<W: ConvertToFmt>   {
    writer: W::Writer,
    last_end: f64,
    last_y: f64,
    first_char: bool,
    flip_ctm: Transform,
}

impl<W: ConvertToFmt> PlainTextOutput<W> {
    pub fn new(writer: W) -> PlainTextOutput<W> {
        PlainTextOutput{
            writer: writer.convert(),
            last_end: 100000.,
            first_char: false,
            last_y: 0.,
            flip_ctm: Transform2D::identity(),
        }
    }
}

impl<W: ConvertToFmt> OutputDev for PlainTextOutput<W> {
    fn begin_page(&mut self, _page_num: u32, media_box: &MediaBox, _: Option<ArtBox>) -> Result<(), OutputError> {
        self.flip_ctm = Transform2D::row_major(1., 0., 0., -1., 0., media_box.ury - media_box.lly);
        Ok(())
    }
    fn end_page(&mut self) -> Result<(), OutputError> {
        Ok(())
    }
    fn output_character(&mut self, trm: &Transform, width: f64, _spacing: f64, font_size: f64, char: &str) -> Result<(), OutputError> {
        let position = trm.post_transform(&self.flip_ctm);
        let transformed_font_size_vec = trm.transform_vector(vec2(font_size, font_size));
        let transformed_font_size = (transformed_font_size_vec.x*transformed_font_size_vec.y).sqrt();
        let (x, y) = (position.m31, position.m32);
        use std::fmt::Write;
        if self.first_char {
            if (y - self.last_y).abs() > transformed_font_size * 1.5 {
                write!(self.writer, "\n")?;
            }

            if x < self.last_end && (y - self.last_y).abs() > transformed_font_size * 0.5 {
                write!(self.writer, "\n")?;
            }

            if x > self.last_end + transformed_font_size * 0.1 {
                dlog!("width: {}, space: {}, thresh: {}", width, x - self.last_end, transformed_font_size * 0.1);
                write!(self.writer, " ")?;
            }
        }
        write!(self.writer, "{}", char)?;
        self.first_char = false;
        self.last_y = y;
        self.last_end = x + width * transformed_font_size;
        Ok(())
    }
    fn begin_word(&mut self) -> Result<(), OutputError> {
        self.first_char = true;
        Ok(())
    }
    fn end_word(&mut self) -> Result<(), OutputError> {Ok(())}
    fn end_line(&mut self) -> Result<(), OutputError>{
        Ok(())
    }
}


pub fn print_metadata(pdf: &Pdf) {
    let metadata = pdf.metadata();
    dlog!("Version: {:?}", pdf.version());
    if let Some(ref title) = metadata.title {
        dlog!("Title: {}", pdf_to_utf8(title));
    }
    if let Some(ref author) = metadata.author {
        dlog!("Author: {}", pdf_to_utf8(author));
    }
    if let Some(ref subject) = metadata.subject {
        dlog!("Subject: {}", pdf_to_utf8(subject));
    }
    if let Some(ref creator) = metadata.creator {
        dlog!("Creator: {}", pdf_to_utf8(creator));
    }
    if let Some(ref producer) = metadata.producer {
        dlog!("Producer: {}", pdf_to_utf8(producer));
    }
    dlog!("Page count: {}", pdf.pages().len());
}

/// Extract the text from a pdf at `path` and return a `String` with the results
pub fn extract_text<P: std::convert::AsRef<std::path::Path>>(path: P) -> Result<String, OutputError> {
    let mut s = String::new();
    {
        let mut output = PlainTextOutput::new(&mut s);
        let data = std::fs::read(path).map_err(|e| OutputError::IoError(e))?;
        let pdf = Pdf::new(Arc::new(data))?;
        output_doc(&pdf, &mut output)?;
    }
    Ok(s)
}

pub fn extract_text_encrypted<P: std::convert::AsRef<std::path::Path>>(
    path: P,
    password: &str,
) -> Result<String, OutputError> {
    let mut s = String::new();
    {
        let mut output = PlainTextOutput::new(&mut s);
        let data = std::fs::read(path).map_err(|e| OutputError::IoError(e))?;
        let pdf = Pdf::new_with_password(Arc::new(data), password)?;
        output_doc(&pdf, &mut output)?;
    }
    Ok(s)
}

pub fn extract_text_from_mem(buffer: &[u8]) -> Result<String, OutputError> {
    let mut s = String::new();
    {
        let mut output = PlainTextOutput::new(&mut s);
        let pdf = Pdf::new(Arc::new(buffer.to_vec()))?;
        output_doc(&pdf, &mut output)?;
    }
    Ok(s)
}

pub fn extract_text_from_mem_encrypted(
    buffer: &[u8],
    password: &str,
) -> Result<String, OutputError> {
    let mut s = String::new();
    {
        let mut output = PlainTextOutput::new(&mut s);
        let pdf = Pdf::new_with_password(Arc::new(buffer.to_vec()), password)?;
        output_doc(&pdf, &mut output)?;
    }
    Ok(s)
}


fn extract_text_by_page(pdf: &Pdf, page_num: u32) -> Result<String, OutputError> {
    let mut s = String::new();
    {
        let mut output = PlainTextOutput::new(&mut s);
        output_doc_page(pdf, &mut output, page_num)?;
    }
    Ok(s)
}

/// Extract the text from a pdf at `path` and return a `Vec<String>` with the results separately by page
pub fn extract_text_by_pages<P: std::convert::AsRef<std::path::Path>>(path: P) -> Result<Vec<String>, OutputError> {
    let mut v = Vec::new();
    {
        let data = std::fs::read(path).map_err(|e| OutputError::IoError(e))?;
        let pdf = Pdf::new(Arc::new(data))?;
        for page_num in 1..=(pdf.pages().len() as u32) {
            let content = extract_text_by_page(&pdf, page_num)?;
            v.push(content);
        }
    }
    Ok(v)
}

pub fn extract_text_by_pages_encrypted<P: std::convert::AsRef<std::path::Path>>(path: P, password: &str) -> Result<Vec<String>, OutputError> {
    let mut v = Vec::new();
    {
        let data = std::fs::read(path).map_err(|e| OutputError::IoError(e))?;
        let pdf = Pdf::new_with_password(Arc::new(data), password)?;
        for page_num in 1..=(pdf.pages().len() as u32) {
            let content = extract_text_by_page(&pdf, page_num)?;
            v.push(content);
        }
    }
    Ok(v)
}

pub fn extract_text_from_mem_by_pages(buffer: &[u8]) -> Result<Vec<String>, OutputError> {
    let mut v = Vec::new();
    {
        let pdf = Pdf::new(Arc::new(buffer.to_vec()))?;
        for page_num in 1..=(pdf.pages().len() as u32) {
            let content = extract_text_by_page(&pdf, page_num)?;
            v.push(content);
        }
    }
    Ok(v)
}

pub fn extract_text_from_mem_by_pages_encrypted(buffer: &[u8], password: &str) -> Result<Vec<String>, OutputError> {
    let mut v = Vec::new();
    {
        let pdf = Pdf::new_with_password(Arc::new(buffer.to_vec()), password)?;
        for page_num in 1..=(pdf.pages().len() as u32) {
            let content = extract_text_by_page(&pdf, page_num)?;
            v.push(content);
        }
    }
    Ok(v)
}


pub fn output_doc_encrypted(
    pdf: &Pdf,
    output: &mut dyn OutputDev,
) -> Result<(), OutputError> {
    // With hayro-syntax, decryption happens at load time via Pdf::new_with_password
    output_doc(pdf, output)
}

/// Parse a given document and output it to `output`
pub fn output_doc(pdf: &Pdf, output: &mut dyn OutputDev) -> Result<(), OutputError> {
    let pages = pdf.pages();
    let mut p = Processor::new();
    for (i, page) in pages.iter().enumerate() {
        let page_num = (i + 1) as u32;
        output_doc_inner(page_num, page, &mut p, output)?;
    }
    Ok(())
}

pub fn output_doc_page(pdf: &Pdf, output: &mut dyn OutputDev, page_num: u32) -> Result<(), OutputError> {
    let pages = pdf.pages();
    let idx = (page_num - 1) as usize;
    if idx >= pages.len() {
        return Err(OutputError::PdfError(LoadPdfError::Invalid));
    }
    let page = &pages[idx];
    let mut p = Processor::new();
    output_doc_inner(page_num, page, &mut p, output)?;
    Ok(())
}

fn output_doc_inner<'a>(page_num: u32, page: &hayro_syntax::page::Page<'a>, p: &mut Processor<'a>, output: &mut dyn OutputDev) -> Result<(), OutputError> {
    let rect = page.media_box();
    let media_box = MediaBox { llx: rect.x0, lly: rect.y0, urx: rect.x1, ury: rect.y1 };
    let art_box: Option<Rect> = page.raw().get::<Rect>(&b"ArtBox"[..]);
    let art_box_tuple = art_box.map(|r| (r.x0, r.y0, r.x1, r.y1));
    output.begin_page(page_num, &media_box, art_box_tuple)?;
    if let Some(content) = page.page_stream() {
        let resources = page.resources();
        p.process_stream(content, resources, &media_box, output, page_num)?;
    }
    output.end_page()?;
    Ok(())
}
