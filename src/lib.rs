extern crate lopdf;

use lopdf::content::Content;
use lopdf::*;
use euclid::*;
use std::fmt::Debug;
extern crate encoding;
extern crate euclid;
extern crate adobe_cmap_parser;
extern crate type1_encoding_parser;
extern crate unicode_normalization;
use euclid::vec2;
use encoding::{Encoding, DecoderTrap};
use encoding::all::UTF_16BE;
use std::fmt;
use std::io::Write;
use std::str;
use std::fs::File;
use std::slice::Iter;
use std::collections::HashMap;
use std::rc::Rc;
mod core_fonts;
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

// an intermediate trait that can be used to chain conversions that may have failed
trait FromOptObj<'a> {
    fn from_opt_obj(doc: &'a Document, obj: Option<&'a Object>, key: &str) -> Self;
}

// conditionally convert to Self returns None if the conversion failed
trait FromObj<'a> where Self: std::marker::Sized {
    fn from_obj(doc: &'a Document, obj: &'a Object) -> Option<Self>;
}

impl<'a, T: FromObj<'a>> FromOptObj<'a> for Option<T> {
    fn from_opt_obj(doc: &'a Document, obj: Option<&'a Object>, key: &str) -> Self {
        obj.and_then(|x| T::from_obj(doc,x))
    }
}

impl<'a, T: FromObj<'a>> FromOptObj<'a> for T {
    fn from_opt_obj(doc: &'a Document, obj: Option<&'a Object>, key: &str) -> Self {
        T::from_obj(doc, obj.expect(key)).expect("wrong type")
    }
}

// we follow the same conventions as pdfium for when to support indirect objects:
// on arrays, streams and dicts
impl<'a, T: FromObj<'a>> FromObj<'a> for Vec<T> {
    fn from_obj(doc: &'a Document, obj: &'a Object) -> Option<Self> {
        maybe_deref(doc, obj).as_array().map(|x| x.iter()
            .map(|x| T::from_obj(doc, x).expect("wrong type"))
            .collect())
    }
}

fn to_array4<T>(v: Vec<T>) -> Option<[T; 4]> {
    if v.len() != 4 { return None }
    let mut i = v.into_iter();
    match (i.next(), i.next(), i.next(), i.next(), i.next()) {
        (Some(a), Some(b), Some(c), Some(d), None) => {
            Some([a, b, c, d])
        }
        _ => None
    }
}

// XXX: These will panic if we don't have the right number of items
// we don't want to do that
impl<'a, T: FromObj<'a>> FromObj<'a> for [T; 4] {
    fn from_obj(doc: &'a Document, obj: &'a Object) -> Option<Self> {
        maybe_deref(doc, obj).as_array().map(|x| {
            let mut all = x.iter()
                .map(|x| T::from_obj(doc, x).expect("wrong type"));
            [all.next().unwrap(), all.next().unwrap(), all.next().unwrap(), all.next().unwrap()]
        })
    }
}

impl<'a, T: FromObj<'a>> FromObj<'a> for [T; 3] {
    fn from_obj(doc: &'a Document, obj: &'a Object) -> Option<Self> {
        maybe_deref(doc, obj).as_array().map(|x| {
            let mut all = x.iter()
                .map(|x| T::from_obj(doc, x).expect("wrong type"));
            [all.next().unwrap(), all.next().unwrap(), all.next().unwrap()]
        })
    }
}

impl<'a> FromObj<'a> for f64 {
    fn from_obj(doc: &Document, obj: &Object) -> Option<Self> {
        match obj {
            &Object::Integer(i) => Some(i as f64),
            &Object::Real(f) => Some(f),
            _ => None
        }
    }
}

impl<'a> FromObj<'a> for i64 {
    fn from_obj(doc: &Document, obj: &Object) -> Option<Self> {
        match obj {
            &Object::Integer(i) => Some(i),
            _ => None
        }
    }
}

impl<'a> FromObj<'a> for &'a Dictionary {
    fn from_obj(doc: &'a Document, obj: &'a Object) -> Option<&'a Dictionary> {
        maybe_deref(doc, obj).as_dict()
    }
}

impl<'a> FromObj<'a> for &'a Object {
    fn from_obj(doc: &'a Document, obj: &'a Object) -> Option<&'a Object> {
        Some(maybe_deref(doc, obj))
    }
}

fn get<'a, T: FromOptObj<'a>>(doc: &'a Document, dict: &'a Dictionary, key: &str) -> T {
    T::from_opt_obj(doc, dict.get(key), key)
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
struct PdfSimpleFont<'a> {
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
        Rc::new(PdfSimpleFont::new(doc, font))
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

impl<'a> PdfSimpleFont<'a> {
    fn new(doc: &'a Document, font: &'a Dictionary) -> PdfSimpleFont<'a> {
        let base_name = get_name(doc, font, "BaseFont");
        let subtype = get_name(doc, font, "Subtype");

        let encoding: Option<&Object> = get(doc, font, "Encoding");
        println!("base_name {} {} enc:{:?} {:?}", base_name, subtype, encoding, font);
        let descriptor: Option<&Dictionary> = get(doc, font, "FontDescriptor");
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
        match encoding {
            Some(&Object::Name(ref encoding_name)) => {
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
            }
            Some(&Object::Dictionary(ref encoding)) => {
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
            None => {
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
            _ => { panic!() }
        }

        let mut width_map = HashMap::new();
        if is_core_font(&base_name) {
            for font_metrics in core_fonts::metrics() {
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
            let mut first_char: i64 = get(doc, font, "FirstChar");
            let mut last_char: i64 = get(doc, font, "LastChar");
            let mut widths: Vec<f64> = get(doc, font, "Widths");
            let mut i = 0;
            println!("first_char {:?}, last_char: {:?}, widths: {} {:?}", first_char, last_char, widths.len(), widths);

            for w in widths {
                width_map.insert((first_char + i), w);
                i += 1;
            }
            assert_eq!(first_char + i - 1, last_char);
        }

        PdfSimpleFont {doc, font, widths: width_map, encoding: encoding_table, default_width: None, unicode_map}
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


impl<'a> PdfFont for PdfSimpleFont<'a> {
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



impl<'a> fmt::Debug for PdfSimpleFont<'a> {
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
        let s = self.to_unicode.as_ref().and_then(|x| x.get(&char));
        if let Some(s) = s {
            let s = [*s as u16];
            let s = String::from_utf16(&s).unwrap();
            s
        } else {
            println!("Unknown character {:?} in {:?}", char, self.font);
            "".to_string()
        }
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

struct Type0Func {
    domain: Vec<f64>,
    range: Vec<f64>,
    contents: Vec<u8>,
    size: Vec<i64>,
    bits_per_sample: i64,
    encode: Vec<f64>,
    decode: Vec<f64>,
}

fn interpolate(x: f64, x_min: f64, x_max: f64, y_min: f64, y_max: f64) -> f64 {
    let divisor = x - x_min;
    if divisor != 0. {
        y_min + (x - x_min) * ((y_max - y_min) / divisor)
    } else {
        // (x - x_min) will be 0 which means we want to discard the interpolation
        // and arbitrarily choose y_min to match pdfium
        y_min
    }
}

impl Type0Func {
    fn eval(&self, input: &[f64], output: &mut [f64]) {
        let n_inputs = self.domain.len() / 2;
        let n_ouputs = self.range.len() / 2;

    }
}

enum Function {
    Type0(Type0Func),
    Type1,
    Type3,
    Type4
}

impl Function {
    fn new(doc: &Document, obj: &Object) -> Function {
        let dict = match obj {
            &Object::Dictionary(ref dict) => dict,
            &Object::Stream(ref stream) => &stream.dict,
            _ => panic!()
        };
        let function_type: i64 = get(doc, dict,"FunctionType");
        let f = match function_type {
            0 => {
                let stream = match obj {
                    &Object::Stream(ref stream) => stream,
                    _ => panic!()
                };
                let range: Vec<f64> = get(doc, dict, "Range");
                let domain: Vec<f64> = get(doc, dict, "Domain");
                let contents = get_contents(stream);
                let size: Vec<i64> = get(doc, dict, "Size");
                let bits_per_sample = get(doc, dict, "BitsPerSample");
                // We ignore 'Order' like pdfium, poppler and pdf.js

                let encode = get::<Option<Vec<f64>>>(doc, dict, "Encode");
                // maybe there's some better way to write this.
                let encode = encode.unwrap_or_else(|| {
                    let mut default = Vec::new();
                    for i in &size {
                        default.extend([0., (i - 1) as f64].iter());
                    }
                    default
                });
                let decode = get::<Option<Vec<f64>>>(doc, dict, "Decode").unwrap_or_else(|| range.clone());

                Function::Type0(Type0Func { domain, range, size, contents, bits_per_sample, encode, decode })
            }
            _ => { panic!() }
        };
        f
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
    leading: f64,
    rise: f64,
    tm: euclid::Transform2D<f64>,
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
    ctm: Transform2D<f64>,
    ts: TextState<'a>,
    smask: Option<&'a Dictionary>,
    fill_colorspace: ColorSpace,
    fill_color: Vec<f64>,
    stroke_colorspace: ColorSpace,
    stroke_color: Vec<f64>,
}

fn show_text(gs: &mut GraphicsState, s: &[u8],
             tlm: &Transform2D<f64>,
             flip_ctm: &Transform2D<f64>,
             output: &mut OutputDev) {
    let ts = &mut gs.ts;
    let font = ts.font.as_ref().unwrap();
    //let encoding = font.encoding.as_ref().map(|x| &x[..]).unwrap_or(&PDFDocEncoding);
    println!("{:?}", font.decode(s));
    output.begin_word();

    for c in font.char_codes(s) {
        let tsm = Transform2D::row_major(ts.font_size * ts.horizontal_scaling,
                                                 0.,
                                                 0.,
                                                 ts.horizontal_scaling,
                                                 0.,
                                                 ts.rise);
        let trm = ts.tm.pre_mul(&gs.ctm);
        let position = trm.post_mul(&flip_ctm);
        //println!("ctm: {:?} tm {:?}", gs.ctm, tm);
        //println!("current pos: {:?}", position);
        // 5.9 Extraction of Text Content


        //println!("w: {}", font.widths[&(*c as i64)]);
        let w0 = font.get_width(c as i64) / 1000.;
        let transformed_font_size_vec = trm.transform_vector(&vec2(ts.font_size, ts.font_size));
        // get the length of one sized of the square with the same area with a rectangle of size (x, y)
        let transformed_font_size = (transformed_font_size_vec.x*transformed_font_size_vec.y).sqrt();
        output.output_character(position.m31, position.m32, w0, transformed_font_size, &font.decode_char(c));
        let tj = 0.;
        let ty = 0.;
        let tx = ts.horizontal_scaling * ((w0 - tj/1000.)* ts.font_size + ts.word_spacing + ts.character_spacing);
        // println!("w0: {}, tx: {}", w0, tx);
        ts.tm = ts.tm.pre_mul(&Transform2D::create_translation(tx, ty));
        let trm = ts.tm.pre_mul(&gs.ctm);
        //println!("post pos: {:?}", trm);

    }
    output.end_word();
}

#[derive(Debug, Clone, Copy)]
pub struct MediaBox {
    llx: f64,
    lly: f64,
    urx: f64,
    ury: f64
}

fn apply_state(gs: &mut GraphicsState, state: &Dictionary) {
    for (k, v) in state.iter() {
        match k.as_ref() {
            "SMask" => { match v {
                &Object::Name(ref name) => {
                    if name == b"None" {
                        gs.smask = None;
                    } else {
                        panic!("unexpected smask name")
                    }
                }
                _ => { panic!("unexpected smask type") }
            }}
            _ => {  println!("unapplied state: {:?} {:?}", k, v); }
        }
    }

}

#[derive(Debug)]
enum PathOp {
    MoveTo(f64, f64),
    LineTo(f64, f64),
    // XXX: is it worth distinguishing the different kinds of curve ops?
    CurveTo(f64, f64, f64, f64, f64, f64),
    Rect(f64, f64, f64, f64),
    Close,
}

#[derive(Debug)]
pub struct Path {
    ops: Vec<PathOp>
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

#[derive(Clone)]
pub struct CalGray {
    white_point: [f64; 3],
    black_point: Option<[f64; 3]>,
    gamma: Option<f64>,
}

#[derive(Clone)]
pub struct CalRGB {
    white_point: [f64; 3],
    black_point: Option<[f64; 3]>,
    gamma: Option<[f64; 3]>,
    matrix: Option<Vec<f64>>
}

#[derive(Clone)]
pub struct Lab {
    white_point: [f64; 3],
    black_point: Option<[f64; 3]>,
    range: Option<[f64; 4]>,
}

#[derive(Clone)]
pub enum ColorSpace {
    DeviceGray,
    DeviceRGB,
    DeviceCMYK,
    CalRGB(CalRGB),
    CalGray(CalGray),
    Lab(Lab),
    Separation,
    ICCBased(Vec<u8>)
}

fn make_colorspace<'a>(doc: &'a Document, name: String, resources: &'a Dictionary) -> ColorSpace {
    match name.as_ref() {
        "DeviceGray" => ColorSpace::DeviceGray,
        "DeviceRGB" => ColorSpace::DeviceRGB,
        "DeviceCMYK" => ColorSpace::DeviceCMYK,
        _ => {
            let colorspaces: &Dictionary = get(&doc, resources, "ColorSpace");
            let cs = maybe_get_array(doc, colorspaces,&name[..]).expect("missing colorspace");
            let cs_name = pdf_to_utf8(cs[0].as_name().expect("first arg must be a name"));
            match cs_name.as_ref() {
                "Separation" => {
                    let name = pdf_to_utf8(cs[1].as_name().expect("second arg must be a name"));
                    let alternate_space = pdf_to_utf8(cs[2].as_name().expect("second arg must be a name"));
                    let tint_transform = maybe_deref(doc, &cs[3]);

                    println!("{:?} {:?} {:?}", name, alternate_space, tint_transform);
                    panic!()
                }
                "ICCBased" => {
                    let stream = maybe_deref(doc, &cs[1]).as_stream().unwrap();
                    println!("ICCBased {:?}", stream);
                    // XXX: we're going to be continually decompressing everytime this object is referenced
                    ColorSpace::ICCBased(get_contents(stream))
                }
                "CalGray" => {
                    let dict = cs[1].as_dict().expect("second arg must be a dict");
                    ColorSpace::CalGray(CalGray {
                        white_point: get(&doc, dict, "WhitePoint"),
                        black_point: get(&doc, dict, "BackPoint"),
                        gamma: get(&doc, dict, "Gamma"),
                    })
                }
                "CalRGB" => {
                    let dict = cs[1].as_dict().expect("second arg must be a dict");
                    ColorSpace::CalRGB(CalRGB {
                        white_point: get(&doc, dict, "WhitePoint"),
                        black_point: get(&doc, dict, "BackPoint"),
                        gamma: get(&doc, dict, "Gamma"),
                        matrix: get(&doc, dict, "Matrix"),
                    })
                }
                "Lab" => {
                    let dict = cs[1].as_dict().expect("second arg must be a dict");
                    ColorSpace::Lab(Lab {
                        white_point: get(&doc, dict, "WhitePoint"),
                        black_point: get(&doc, dict, "BackPoint"),
                        range: get(&doc, dict, "Range"),
                    })
                }
                _ => {
                    println!("color_space {} {:?} {:?}", name, cs_name, cs);

                    panic!()
                }
            }
        }
    }
}

fn process_stream(doc: &Document, content: Vec<u8>, resources: &Dictionary, media_box: &MediaBox, output: &mut OutputDev, page_num: u32) {
    let content = Content::decode(&content).unwrap();
    let mut font_table = HashMap::new();
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
        ctm: Transform2D::identity(),
        smask: None
    };
    //let mut ts = &mut gs.ts;
    let mut gs_stack = Vec::new();
    let mut mc_stack = Vec::new();
    // XXX: replace tlm with a point for text start
    let mut tlm = Transform2D::identity();
    let mut path = Path::new();
    let mut flip_ctm = Transform2D::row_major(1., 0., 0., -1., 0., (media_box.ury - media_box.lly));
    println!("MediaBox {:?}", media_box);
    for operation in &content.operations {
        //println!("op: {:?}", operation);
        match operation.operator.as_ref() {
            "BT" => {
                tlm = Transform2D::identity();
                gs.ts.tm = tlm;
            }
            "ET" => {
                tlm = Transform2D::identity();
                gs.ts.tm = tlm;
            }
            "cm" => {
                assert!(operation.operands.len() == 6);
                let m = Transform2D::row_major(as_num(&operation.operands[0]),
                                                       as_num(&operation.operands[1]),
                                                       as_num(&operation.operands[2]),
                                                       as_num(&operation.operands[3]),
                                                       as_num(&operation.operands[4]),
                                                       as_num(&operation.operands[5]));
                gs.ctm = gs.ctm.pre_mul(&m);
                println!("matrix {:?}", gs.ctm);
            }
            "CS" => {
                let name = pdf_to_utf8(operation.operands[0].as_name().unwrap());
                gs.stroke_colorspace = make_colorspace(doc, name, resources);
            }
            "cs" => {
                let name = pdf_to_utf8(operation.operands[0].as_name().unwrap());
                gs.fill_colorspace = make_colorspace(doc, name, resources);
            }
            "SC" | "SCN" => {
                gs.stroke_color = operation.operands.iter().map(|x| as_num(x)).collect();
            }
            "sc" | "scn" => {
                gs.fill_color = operation.operands.iter().map(|x| as_num(x)).collect();
            }
            "G" | "g" | "RG" | "rg" | "K" | "k" => {
                println!("unhandled color operation {:?}", operation);
            }
            "TJ" => {
                match operation.operands[0] {
                    Object::Array(ref array) => {
                        for e in array {
                            match e {
                                &Object::String(ref s, _) => {
                                    show_text(&mut gs, s, &tlm, &flip_ctm, output);
                                }
                                &Object::Integer(i) => {
                                    let ts = &mut gs.ts;
                                    let w0 = 0.;
                                    let tj = i as f64;
                                    let ty = 0.;
                                    let tx = ts.horizontal_scaling * ((w0 - tj/1000.)* ts.font_size + ts.word_spacing + ts.character_spacing);
                                    ts.tm = ts.tm.pre_mul(&Transform2D::create_translation(tx, ty));
                                    println!("adjust text by: {} {:?}", i, ts.tm);
                                }
                                &Object::Real(i) => {
                                    let ts = &mut gs.ts;
                                    let w0 = 0.;
                                    let tj = i as f64;
                                    let ty = 0.;
                                    let tx = ts.horizontal_scaling * ((w0 - tj/1000.)* ts.font_size + ts.word_spacing + ts.character_spacing);
                                    ts.tm = ts.tm.pre_mul(&Transform2D::create_translation(tx, ty));
                                    println!("adjust text by: {} {:?}", i, ts.tm);
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
                        show_text(&mut gs, s, &tlm, &flip_ctm, output);
                    }
                    _ => { panic!("unexpected Tj operand {:?}", operation) }
                }
            }
            "Tc" => {
                gs.ts.character_spacing = as_num(&operation.operands[0]);
            }
            "Tw" => {
                gs.ts.word_spacing = as_num(&operation.operands[0]);
            }
            "Tz" => {
                gs.ts.horizontal_scaling = as_num(&operation.operands[0]) / 100.;
            }
            "Tf" => {
                let fonts: &Dictionary = get(&doc, resources, "Font");
                let name = str::from_utf8(operation.operands[0].as_name().unwrap()).unwrap();
                let font = font_table.entry(name).or_insert_with(|| make_font(doc, get_obj(doc, fonts.get(name).unwrap()).as_dict().unwrap())).clone();
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
                println!("font size: {} {:?}", gs.ts.font_size, operation);
            }
            "Ts" => {
                gs.ts.rise = as_num(&operation.operands[0]);
            }
            "Tm" => {
                assert!(operation.operands.len() == 6);
                tlm = Transform2D::row_major(as_num(&operation.operands[0]),
                                                       as_num(&operation.operands[1]),
                                                       as_num(&operation.operands[2]),
                                                       as_num(&operation.operands[3]),
                                                       as_num(&operation.operands[4]),
                                                       as_num(&operation.operands[5]));
                gs.ts.tm = tlm;
                println!("Tm: matrix {:?}", gs.ts.tm);
                output.end_line();

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

                tlm = tlm.pre_mul(&Transform2D::create_translation(tx, ty));
                gs.ts.tm = tlm;
                println!("Td matrix {:?}", gs.ts.tm);
                output.end_line();

            }
            "T*" => {
                let tx = 0.0;
                let ty = gs.ts.leading;

                tlm = tlm.pre_mul(&Transform2D::create_translation(tx, ty));
                gs.ts.tm = tlm;
                println!("Td matrix {:?}", gs.ts.tm);
                output.end_line();
            }
            "q" => { gs_stack.push(gs.clone()); }
            "Q" => { gs = gs_stack.pop().unwrap(); }
            "gs" => {
                let ext_gstate: &Dictionary = get(doc, resources, "ExtGState");
                let name = pdf_to_utf8(operation.operands[0].as_name().unwrap());
                let state: &Dictionary = get(doc, ext_gstate, &name.clone());
                apply_state(&mut gs, state);
            }
            "w" | "J" | "j" | "M" | "d" | "ri" | "i" => { println!("unknown graphics state operator {:?}", operation); }
            "m" => { path.ops.push(PathOp::MoveTo(as_num(&operation.operands[0]), as_num(&operation.operands[1]))) }
            "l" => { path.ops.push(PathOp::LineTo(as_num(&operation.operands[0]), as_num(&operation.operands[1]))) }
            "c" => { path.ops.push(PathOp::CurveTo(
                as_num(&operation.operands[0]),
                as_num(&operation.operands[1]),
                as_num(&operation.operands[2]),
                as_num(&operation.operands[3]),
                as_num(&operation.operands[4]),
                as_num(&operation.operands[5])))
            }
            "v" => {
                let (x, y) = path.current_point();
                path.ops.push(PathOp::CurveTo(
                x,
                y,
                as_num(&operation.operands[0]),
                as_num(&operation.operands[1]),
                as_num(&operation.operands[2]),
                as_num(&operation.operands[3])))
            }
            "y" => { path.ops.push(PathOp::CurveTo(
                as_num(&operation.operands[0]),
                as_num(&operation.operands[1]),
                as_num(&operation.operands[2]),
                as_num(&operation.operands[3]),
                as_num(&operation.operands[2]),
                as_num(&operation.operands[3])))
            }
            "h" => { path.ops.push(PathOp::Close) }
            "re" => {
                path.ops.push(PathOp::Rect(as_num(&operation.operands[0]),
                                           as_num(&operation.operands[1]),
                                           as_num(&operation.operands[2]),
                                           as_num(&operation.operands[3])))
            }
            "s" | "f*" | "B" | "B*" | "b" | "n" => {
                println!("unhandled path op {:?}", operation);
            }
            "S" => {
                output.stroke(&gs.ctm, &gs.stroke_colorspace, &gs.stroke_color, &path);
                path.ops.clear();
            }
            "F" | "f" => {
                output.fill(&gs.ctm, &gs.fill_colorspace, &gs.fill_color, &path);
                path.ops.clear();
            }
            "W" | "w*" => { println!("unhandled clipping operation {:?}", operation); }
            "n" => {
                println!("discard {:?}", path);
                path.ops.clear();
            }
            "BMC" | "BDC" => {
                mc_stack.push(operation);
            }
            "EMC" => {
                mc_stack.pop();
            }
            _ => { println!("unknown operation {:?}", operation);}
        }
    }

}


pub trait OutputDev {
    fn begin_page(&mut self, page_num: u32, media_box: &MediaBox, art_box: Option<(f64, f64, f64, f64)>);
    fn end_page(&mut self);
    fn output_character(&mut self, x: f64, y: f64, width: f64, font_size: f64, char: &str);
    fn begin_word(&mut self);
    fn end_word(&mut self);
    fn end_line(&mut self);
    fn stroke(&mut self, ctm: &Transform2D<f64>, colorspace: &ColorSpace, color: &[f64], &Path) {}
    fn fill(&mut self, ctm: &Transform2D<f64>, colorspace: &ColorSpace, color: &[f64], &Path) {}
}


pub struct HTMLOutput<'a>  {
    file: &'a mut File
}

impl<'a> HTMLOutput<'a> {
    pub fn new(file: &mut File) -> HTMLOutput {
        HTMLOutput{file}
    }
}

type ArtBox = (f64, f64, f64, f64);

impl<'a> OutputDev for HTMLOutput<'a> {
    fn begin_page(&mut self, page_num: u32, media_box: &MediaBox, _: Option<ArtBox>) {
        write!(self.file, "<meta charset='utf-8' /> ");
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

pub struct SVGOutput<'a>  {
    file: &'a mut File
}
impl<'a> SVGOutput<'a> {
    pub fn new(file: &mut File) -> SVGOutput {
        SVGOutput{file}
    }
}

impl<'a> OutputDev for SVGOutput<'a> {
    fn begin_page(&mut self, page_num: u32, media_box: &MediaBox, art_box: Option<(f64, f64, f64, f64)>) {
        let ver = 1.1;
        write!(self.file, "<?xml version=\"1.0\" encoding=\"UTF-8\" ?>\n");
        if ver == 1.1 {
            write!(self.file, r#"<!DOCTYPE svg PUBLIC "-//W3C//DTD SVG 1.1//EN" "http://www.w3.org/Graphics/SVG/1.1/DTD/svg11.dtd">"#);
        } else {
            write!(self.file, r#"<!DOCTYPE svg PUBLIC "-//W3C//DTD SVG 1.0//EN" "http://www.w3.org/TR/2001/REC-SVG-20010904/DTD/svg10.dtd">"#);
        }
        if let Some(art_box) = art_box {
            let width = art_box.2 - art_box.0;
            let height = art_box.3 - art_box.1;
            let y = media_box.ury - art_box.1 - height;
            write!(self.file, "<svg width=\"{}\" height=\"{}\" xmlns=\"http://www.w3.org/2000/svg\" version=\"{}\" viewBox='{} {} {} {}'>", width, height, ver, art_box.0, y, width, height);
        } else {
            let width = media_box.urx - media_box.llx;
            let height = media_box.ury - media_box.lly;
            write!(self.file, "<svg width=\"{}\" height=\"{}\" xmlns=\"http://www.w3.org/2000/svg\" version=\"{}\" viewBox='{} {} {} {}'>", width, height, ver, media_box.llx, media_box.lly, width, height);
        }
        write!(self.file, "\n");
        type Mat = Transform2D<f64>;

        let mut ctm = Mat::create_scale(1., -1.).post_translate(vec2(0., media_box.ury));
        write!(self.file, "<g transform='matrix({}, {}, {}, {}, {}, {})'>\n",
               ctm.m11,
               ctm.m12,
               ctm.m21,
               ctm.m22,
               ctm.m31,
               ctm.m32,
        );
    }
    fn end_page(&mut self) {
        write!(self.file, "</g>\n");
        write!(self.file, "</svg>");
    }
    fn output_character(&mut self, x: f64, y: f64, width: f64, font_size: f64, char: &str) {
    }
    fn begin_word(&mut self) {}
    fn end_word(&mut self) {}
    fn end_line(&mut self) {}
    fn fill(&mut self, ctm: &Transform2D<f64>, _colorspace: &ColorSpace, _color: &[f64], path: &Path) {
        write!(self.file, "<g transform='matrix({}, {}, {}, {}, {}, {})'>",
               ctm.m11,
               ctm.m12,
               ctm.m21,
               ctm.m22,
               ctm.m31,
               ctm.m32,
        );

        /*if path.ops.len() == 1 {
            if let PathOp::Rect(x, y, width, height) = path.ops[0] {
                write!(self.file, "<rect x={} y={} width={} height={} />\n", x, y, width, height);
                write!(self.file, "</g>");
                return;
            }
        }*/
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
        write!(self.file, "<path d='{}' />", d.join(" "));
        write!(self.file, "</g>");
        write!(self.file, "\n");

    }
}

pub struct PlainTextOutput<'a>  {
    file: &'a mut File,
    last_end: f64,
    last_y: f64,
    first_char: bool
}

impl<'a> PlainTextOutput<'a> {
    pub fn new(file: &mut File) -> PlainTextOutput {
        PlainTextOutput{file, last_end: 100000., first_char: false, last_y: 0.}
    }
}

/* There are some structural hints that PDFs can use to signal word and line endings:
 * however relying on these is not likely to be sufficient. */
impl<'a> OutputDev for PlainTextOutput<'a> {
    fn begin_page(&mut self, page_num: u32, media_box: &MediaBox, _: Option<ArtBox>) {
    }
    fn end_page(&mut self) {
    }
    fn output_character(&mut self, x: f64, y: f64, width: f64, font_size: f64, char: &str) {
        //println!("last_end: {} x: {}, width: {}", self.last_end, x, width);
        if self.first_char {
            if (y - self.last_y).abs() > font_size * 1.5 {
                write!(self.file, "\n");
            }
            if x < self.last_end && (y - self.last_y).abs() > font_size * 0.5 {
                write!(self.file, "\n");
            }

            if x > self.last_end + font_size * 0.1 {
                println!("width: {}, space: {}, thresh: {}", width, x - self.last_end, font_size * 0.1);
                write!(self.file, " ");
            }
        }
        //let norm = unicode_normalization::UnicodeNormalization::nfkc(char);
        write!(self.file, "{}", char);
        self.first_char = false;
        self.last_y = y;
        self.last_end = x + width * font_size;
    }
    fn begin_word(&mut self) {
        self.first_char = true;
    }
    fn end_word(&mut self) {}
    fn end_line(&mut self) {
        //write!(self.file, "\n");
    }
}


pub fn print_metadata(doc: &Document) {
    println!("Version: {}", doc.version);
    if let Some(ref info) = get_info(&doc) {
        for (k, v) in *info {
            match v {
                &Object::String(ref s, StringFormat::Literal) => { println!("{}: {}", k, pdf_to_utf8(s)); }
                _ => {}
            }
        }
    }
    println!("Page count: {}", get::<i64>(&doc, &get_pages(&doc), "Count"));
    println!("Pages: {:?}", get_pages(&doc));
    println!("Type: {:?}", get_pages(&doc).get("Type").and_then(|x| x.as_name()).unwrap());
}


/// Parse a given document and output it to `output`
pub fn output_doc(doc: &Document, output: &mut OutputDev) {

    let pages = doc.get_pages();
    for dict in pages {
        let page_num = dict.0;
        let page_dict = doc.get_object(dict.1).unwrap().as_dict().unwrap();
        println!("page {} {:?}", page_num, page_dict);
        let resources: &Dictionary = get(doc, page_dict, "Resources");
        println!("resources {:?}", resources);

        // pdfium searches up the page tree for MediaBoxes as needed
        let media_box = get::<Option<Vec<f64>>>(doc, page_dict, "MediaBox")
            .or_else(|| get::<Option<Vec<f64>>>(&doc, get_pages(&doc), "MediaBox"))
            .map(|media_box| MediaBox { llx: media_box[0], lly: media_box[1], urx: media_box[2], ury: media_box[3] })
            .expect("Should have been a MediaBox");

        let art_box = get::<Option<Vec<f64>>>(&doc, page_dict, "ArtBox")
            .map(|x| (x[0], x[1], x[2], x[3]));

        output.begin_page(page_num, &media_box, art_box);

        process_stream(&doc, doc.get_page_content(dict.1).unwrap(), resources,&media_box, output, page_num);

        output.end_page();
    }
}
