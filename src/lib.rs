extern crate adobe_cmap_parser;
extern crate encoding_rs;
extern crate euclid;
extern crate lopdf;
extern crate type1_encoding_parser;
extern crate unicode_normalization;

use adobe_cmap_parser::{ByteMapping, CIDRange, CodeRange};
use encoding_rs::UTF_16BE;
use euclid::vec2;
use euclid::Transform2D;
use lopdf::{content::Content, encryption::DecryptionError};
pub use lopdf::{Dictionary, Document, Error, Object, ObjectId, Stream, StringFormat};
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::fmt;
use std::fmt::Write;
use std::fmt::{Debug, Formatter};
use std::fs::File;
use std::marker::PhantomData;
use std::rc::Rc;
use std::result::Result;
use std::slice::Iter;
use unicode_normalization::UnicodeNormalization;

mod core_fonts;
mod encodings;
mod glyphnames;
mod zapfglyphnames;

pub struct Space;
pub type Transform = Transform2D<f64, Space, Space>;

#[derive(Debug)]
pub enum OutputError {
    FormatError(std::fmt::Error),
    IoError(std::io::Error),
    PdfError(lopdf::Error),
}

impl std::fmt::Display for OutputError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            Self::FormatError(e) => write!(f, "Formating error: {e}"),
            Self::IoError(e) => write!(f, "IO error: {e}"),
            Self::PdfError(e) => write!(f, "PDF error: {e}"),
        }
    }
}

impl std::error::Error for OutputError {}

impl From<std::fmt::Error> for OutputError {
    fn from(e: std::fmt::Error) -> Self {
        Self::FormatError(e)
    }
}

impl From<std::io::Error> for OutputError {
    fn from(e: std::io::Error) -> Self {
        Self::IoError(e)
    }
}

impl From<lopdf::Error> for OutputError {
    fn from(e: lopdf::Error) -> Self {
        Self::PdfError(e)
    }
}

macro_rules! dlog {
    ($($e:expr),*) => { {$(let _ = $e;)*} }
    //($($t:tt)*) => { println!($($t)*) }
}

fn get_info(doc: &Document) -> Option<&Dictionary> {
    if let Ok(Object::Reference(id)) = doc.trailer.get(b"Info") {
        if let Ok(Object::Dictionary(info)) = doc.get_object(*id) {
            return Some(info);
        }
    }
    None
}

fn get_catalog(doc: &Document) -> &Dictionary {
    if let Object::Reference(id) = doc.trailer.get(b"Root").unwrap() {
        if let Ok(Object::Dictionary(catalog)) = doc.get_object(*id) {
            return catalog;
        }
    }
    panic!();
}

fn get_pages(doc: &Document) -> &Dictionary {
    let catalog = get_catalog(doc);
    match catalog.get(b"Pages").unwrap() {
        Object::Reference(id) => match doc.get_object(*id) {
            Ok(Object::Dictionary(pages)) => {
                return pages;
            }
            other => {
                dlog!("pages: {:?}", other);
            }
        },
        other => {
            dlog!("pages: {:?}", other);
        }
    }
    dlog!("catalog {:?}", catalog);
    panic!();
}

#[allow(non_upper_case_globals)]
const PDFDocEncoding: &[u16] = &[
    0x0000, 0x0001, 0x0002, 0x0003, 0x0004, 0x0005, 0x0006, 0x0007, 0x0008, 0x0009, 0x000a, 0x000b,
    0x000c, 0x000d, 0x000e, 0x000f, 0x0010, 0x0011, 0x0012, 0x0013, 0x0014, 0x0015, 0x0016, 0x0017,
    0x02d8, 0x02c7, 0x02c6, 0x02d9, 0x02dd, 0x02db, 0x02da, 0x02dc, 0x0020, 0x0021, 0x0022, 0x0023,
    0x0024, 0x0025, 0x0026, 0x0027, 0x0028, 0x0029, 0x002a, 0x002b, 0x002c, 0x002d, 0x002e, 0x002f,
    0x0030, 0x0031, 0x0032, 0x0033, 0x0034, 0x0035, 0x0036, 0x0037, 0x0038, 0x0039, 0x003a, 0x003b,
    0x003c, 0x003d, 0x003e, 0x003f, 0x0040, 0x0041, 0x0042, 0x0043, 0x0044, 0x0045, 0x0046, 0x0047,
    0x0048, 0x0049, 0x004a, 0x004b, 0x004c, 0x004d, 0x004e, 0x004f, 0x0050, 0x0051, 0x0052, 0x0053,
    0x0054, 0x0055, 0x0056, 0x0057, 0x0058, 0x0059, 0x005a, 0x005b, 0x005c, 0x005d, 0x005e, 0x005f,
    0x0060, 0x0061, 0x0062, 0x0063, 0x0064, 0x0065, 0x0066, 0x0067, 0x0068, 0x0069, 0x006a, 0x006b,
    0x006c, 0x006d, 0x006e, 0x006f, 0x0070, 0x0071, 0x0072, 0x0073, 0x0074, 0x0075, 0x0076, 0x0077,
    0x0078, 0x0079, 0x007a, 0x007b, 0x007c, 0x007d, 0x007e, 0x0000, 0x2022, 0x2020, 0x2021, 0x2026,
    0x2014, 0x2013, 0x0192, 0x2044, 0x2039, 0x203a, 0x2212, 0x2030, 0x201e, 0x201c, 0x201d, 0x2018,
    0x2019, 0x201a, 0x2122, 0xfb01, 0xfb02, 0x0141, 0x0152, 0x0160, 0x0178, 0x017d, 0x0131, 0x0142,
    0x0153, 0x0161, 0x017e, 0x0000, 0x20ac, 0x00a1, 0x00a2, 0x00a3, 0x00a4, 0x00a5, 0x00a6, 0x00a7,
    0x00a8, 0x00a9, 0x00aa, 0x00ab, 0x00ac, 0x0000, 0x00ae, 0x00af, 0x00b0, 0x00b1, 0x00b2, 0x00b3,
    0x00b4, 0x00b5, 0x00b6, 0x00b7, 0x00b8, 0x00b9, 0x00ba, 0x00bb, 0x00bc, 0x00bd, 0x00be, 0x00bf,
    0x00c0, 0x00c1, 0x00c2, 0x00c3, 0x00c4, 0x00c5, 0x00c6, 0x00c7, 0x00c8, 0x00c9, 0x00ca, 0x00cb,
    0x00cc, 0x00cd, 0x00ce, 0x00cf, 0x00d0, 0x00d1, 0x00d2, 0x00d3, 0x00d4, 0x00d5, 0x00d6, 0x00d7,
    0x00d8, 0x00d9, 0x00da, 0x00db, 0x00dc, 0x00dd, 0x00de, 0x00df, 0x00e0, 0x00e1, 0x00e2, 0x00e3,
    0x00e4, 0x00e5, 0x00e6, 0x00e7, 0x00e8, 0x00e9, 0x00ea, 0x00eb, 0x00ec, 0x00ed, 0x00ee, 0x00ef,
    0x00f0, 0x00f1, 0x00f2, 0x00f3, 0x00f4, 0x00f5, 0x00f6, 0x00f7, 0x00f8, 0x00f9, 0x00fa, 0x00fb,
    0x00fc, 0x00fd, 0x00fe, 0x00ff,
];

fn pdf_to_utf8(s: &[u8]) -> String {
    if s.len() > 2 && s[0] == 0xfe && s[1] == 0xff {
        UTF_16BE
            .decode_without_bom_handling_and_without_replacement(&s[2..])
            .unwrap()
            .to_string()
    } else {
        let r: Vec<u8> = s
            .iter()
            .copied()
            .flat_map(|x| {
                let k = PDFDocEncoding[x as usize];
                vec![(k >> 8) as u8, k as u8].into_iter()
            })
            .collect();
        UTF_16BE
            .decode_without_bom_handling_and_without_replacement(&r)
            .unwrap()
            .to_string()
    }
}

fn to_utf8(encoding: &[u16], s: &[u8]) -> String {
    if s.len() > 2 && s[0] == 0xfe && s[1] == 0xff {
        UTF_16BE
            .decode_without_bom_handling_and_without_replacement(&s[2..])
            .unwrap()
            .to_string()
    } else {
        let r: Vec<u8> = s
            .iter()
            .copied()
            .flat_map(|x| {
                let k = encoding[x as usize];
                vec![(k >> 8) as u8, k as u8].into_iter()
            })
            .collect();
        UTF_16BE
            .decode_without_bom_handling_and_without_replacement(&r)
            .unwrap()
            .to_string()
    }
}

fn maybe_deref<'a>(doc: &'a Document, o: &'a Object) -> &'a Object {
    match o {
        &Object::Reference(r) => doc.get_object(r).expect("missing object reference"),
        _ => o,
    }
}

fn maybe_get_obj<'a>(doc: &'a Document, dict: &'a Dictionary, key: &[u8]) -> Option<&'a Object> {
    dict.get(key).map(|o| maybe_deref(doc, o)).ok()
}

// an intermediate trait that can be used to chain conversions that may have failed
trait FromOptObj<'a> {
    fn from_opt_obj(doc: &'a Document, obj: Option<&'a Object>, key: &[u8]) -> Self;
}

// conditionally convert to Self returns None if the conversion failed
trait FromObj<'a>
where
    Self: std::marker::Sized,
{
    fn from_obj(doc: &'a Document, obj: &'a Object) -> Option<Self>;
}

impl<'a, T: FromObj<'a>> FromOptObj<'a> for Option<T> {
    fn from_opt_obj(doc: &'a Document, obj: Option<&'a Object>, _key: &[u8]) -> Self {
        obj.and_then(|x| T::from_obj(doc, x))
    }
}

impl<'a, T: FromObj<'a>> FromOptObj<'a> for T {
    fn from_opt_obj(doc: &'a Document, obj: Option<&'a Object>, key: &[u8]) -> Self {
        T::from_obj(
            doc,
            obj.unwrap_or_else(|| panic!("{}", String::from_utf8_lossy(key).to_string())),
        )
        .expect("wrong type")
    }
}

// we follow the same conventions as pdfium for when to support indirect objects:
// on arrays, streams and dicts
impl<'a, T: FromObj<'a>> FromObj<'a> for Vec<T> {
    fn from_obj(doc: &'a Document, obj: &'a Object) -> Option<Self> {
        maybe_deref(doc, obj)
            .as_array()
            .map(|x| {
                x.iter()
                    .map(|x| T::from_obj(doc, x).expect("wrong type"))
                    .collect()
            })
            .ok()
    }
}

// XXX: These will panic if we don't have the right number of items
// we don't want to do that
impl<'a, T: FromObj<'a>> FromObj<'a> for [T; 4] {
    fn from_obj(doc: &'a Document, obj: &'a Object) -> Option<Self> {
        maybe_deref(doc, obj)
            .as_array()
            .map(|x| {
                let mut all = x.iter().map(|x| T::from_obj(doc, x).expect("wrong type"));
                [
                    all.next().unwrap(),
                    all.next().unwrap(),
                    all.next().unwrap(),
                    all.next().unwrap(),
                ]
            })
            .ok()
    }
}

impl<'a, T: FromObj<'a>> FromObj<'a> for [T; 3] {
    fn from_obj(doc: &'a Document, obj: &'a Object) -> Option<Self> {
        maybe_deref(doc, obj)
            .as_array()
            .map(|x| {
                let mut all = x.iter().map(|x| T::from_obj(doc, x).expect("wrong type"));
                [
                    all.next().unwrap(),
                    all.next().unwrap(),
                    all.next().unwrap(),
                ]
            })
            .ok()
    }
}

impl FromObj<'_> for f64 {
    fn from_obj(_doc: &Document, obj: &Object) -> Option<Self> {
        match *obj {
            Object::Integer(i) => Some(i as Self),
            Object::Real(f) => Some(f.into()),
            _ => None,
        }
    }
}

impl FromObj<'_> for i64 {
    fn from_obj(_doc: &Document, obj: &Object) -> Option<Self> {
        match obj {
            &Object::Integer(i) => Some(i),
            _ => None,
        }
    }
}

impl<'a> FromObj<'a> for &'a Dictionary {
    fn from_obj(doc: &'a Document, obj: &'a Object) -> Option<&'a Dictionary> {
        maybe_deref(doc, obj).as_dict().ok()
    }
}

impl<'a> FromObj<'a> for &'a Stream {
    fn from_obj(doc: &'a Document, obj: &'a Object) -> Option<&'a Stream> {
        maybe_deref(doc, obj).as_stream().ok()
    }
}

impl<'a> FromObj<'a> for &'a Object {
    fn from_obj(doc: &'a Document, obj: &'a Object) -> Option<&'a Object> {
        Some(maybe_deref(doc, obj))
    }
}

fn get<'a, T: FromOptObj<'a>>(doc: &'a Document, dict: &'a Dictionary, key: &[u8]) -> T {
    T::from_opt_obj(doc, dict.get(key).ok(), key)
}

fn maybe_get<'a, T: FromObj<'a>>(doc: &'a Document, dict: &'a Dictionary, key: &[u8]) -> Option<T> {
    maybe_get_obj(doc, dict, key).and_then(|o| T::from_obj(doc, o))
}

fn get_name_string<'a>(doc: &'a Document, dict: &'a Dictionary, key: &[u8]) -> String {
    pdf_to_utf8(
        dict.get(key)
            .map_or_else(|_| panic!("deref"), |o| maybe_deref(doc, o))
            .as_name()
            .expect("name"),
    )
}

#[allow(dead_code)]
fn maybe_get_name_string<'a>(
    doc: &'a Document,
    dict: &'a Dictionary,
    key: &[u8],
) -> Option<String> {
    maybe_get_obj(doc, dict, key)
        .and_then(|n| n.as_name().ok())
        .map(pdf_to_utf8)
}

fn maybe_get_name<'a>(doc: &'a Document, dict: &'a Dictionary, key: &[u8]) -> Option<&'a [u8]> {
    maybe_get_obj(doc, dict, key).and_then(|n| n.as_name().ok())
}

fn maybe_get_array<'a>(
    doc: &'a Document,
    dict: &'a Dictionary,
    key: &[u8],
) -> Option<&'a Vec<Object>> {
    maybe_get_obj(doc, dict, key).and_then(|n| n.as_array().ok())
}

#[derive(Clone)]
struct PdfSimpleFont<'a> {
    font: &'a Dictionary,
    doc: &'a Document,
    encoding: Option<Vec<u16>>,
    unicode_map: Option<HashMap<u32, String>>,
    widths: HashMap<CharCode, f64>, // should probably just use i32 here
    missing_width: f64,
}

#[derive(Clone)]
struct PdfType3Font<'a> {
    font: &'a Dictionary,
    doc: &'a Document,
    encoding: Option<Vec<u16>>,
    unicode_map: Option<HashMap<u32, String>>,
    widths: HashMap<CharCode, f64>, // should probably just use i32 here
}

fn make_font<'a>(doc: &'a Document, font: &'a Dictionary) -> Rc<dyn PdfFont + 'a> {
    let subtype = get_name_string(doc, font, b"Subtype");
    dlog!("MakeFont({})", subtype);
    if subtype == "Type0" {
        Rc::new(PdfCIDFont::new(doc, font))
    } else if subtype == "Type3" {
        Rc::new(PdfType3Font::new(doc, font))
    } else {
        Rc::new(PdfSimpleFont::new(doc, font))
    }
}

fn is_core_font(name: &str) -> bool {
    matches!(
        name,
        "Courier-Bold"
            | "Courier-BoldOblique"
            | "Courier-Oblique"
            | "Courier"
            | "Helvetica-Bold"
            | "Helvetica-BoldOblique"
            | "Helvetica-Oblique"
            | "Helvetica"
            | "Symbol"
            | "Times-Bold"
            | "Times-BoldItalic"
            | "Times-Italic"
            | "Times-Roman"
            | "ZapfDingbats"
    )
}

fn encoding_to_unicode_table(name: &[u8]) -> Vec<u16> {
    let encoding = match name {
        b"MacRomanEncoding" => encodings::MAC_ROMAN_ENCODING,
        b"MacExpertEncoding" => encodings::MAC_EXPERT_ENCODING,
        b"WinAnsiEncoding" => encodings::WIN_ANSI_ENCODING,
        _ => panic!("unexpected encoding {:?}", pdf_to_utf8(name)),
    };
    let encoding_table = encoding
        .iter()
        .map(|x| {
            if let &Some(x) = x {
                glyphnames::name_to_unicode(x).unwrap()
            } else {
                0
            }
        })
        .collect();
    encoding_table
}

/* "Glyphs in the font are selected by single-byte character codes obtained from a string that
    is shown by the text-showing operators. Logically, these codes index into a table of 256
    glyphs; the mapping from codes to glyphs is called the font’s encoding. Each font program
    has a built-in encoding. Under some circumstances, the encoding can be altered by means
    described in Section 5.5.5, “Character Encoding.”
*/
impl<'a> PdfSimpleFont<'a> {
    fn new(doc: &'a Document, font: &'a Dictionary) -> Self {
        let base_name = get_name_string(doc, font, b"BaseFont");
        let subtype = get_name_string(doc, font, b"Subtype");

        let encoding: Option<&Object> = get(doc, font, b"Encoding");
        dlog!(
            "base_name {} {} enc:{:?} {:?}",
            base_name,
            subtype,
            encoding,
            font
        );
        let descriptor: Option<&Dictionary> = get(doc, font, b"FontDescriptor");
        let mut type1_encoding = None;
        if let Some(descriptor) = descriptor {
            dlog!("descriptor {:?}", descriptor);
            if subtype == "Type1" {
                let file = maybe_get_obj(doc, descriptor, b"FontFile");
                match file {
                    Some(Object::Stream(s)) => {
                        let s = get_contents(s);
                        //dlog!("font contents {:?}", pdf_to_utf8(&s));
                        type1_encoding =
                            Some(type1_encoding_parser::get_encoding_map(&s).expect("encoding"));
                    }
                    _ => {
                        dlog!("font file {:?}", file);
                    }
                }
            } else if subtype == "TrueType" {
                let file = maybe_get_obj(doc, descriptor, b"FontFile2");
                match file {
                    Some(Object::Stream(s)) => {
                        let _s = get_contents(s);
                        //File::create(format!("/tmp/{}", base_name)).unwrap().write_all(&s);
                    }
                    _ => {
                        dlog!("font file {:?}", file);
                    }
                }
            }

            let font_file3 = get::<Option<&Object>>(doc, descriptor, b"FontFile3");
            match font_file3 {
                Some(Object::Stream(s)) => {
                    let subtype = get_name_string(doc, &s.dict, b"Subtype");
                    dlog!("font file {}, {:?}", subtype, s);
                }
                None => {}
                _ => {
                    dlog!("unexpected");
                }
            }

            let charset = maybe_get_obj(doc, descriptor, b"CharSet");
            let _charset = match charset {
                Some(Object::String(s, _)) => Some(pdf_to_utf8(s)),
                _ => None,
            };
            //dlog!("charset {:?}", charset);
        }

        let mut unicode_map = get_unicode_map(doc, font);

        let mut encoding_table = None;
        match encoding {
            Some(Object::Name(encoding_name)) => {
                dlog!("encoding {:?}", pdf_to_utf8(encoding_name));
                encoding_table = Some(encoding_to_unicode_table(encoding_name));
            }
            Some(Object::Dictionary(encoding)) => {
                //dlog!("Encoding {:?}", encoding);
                let mut table = maybe_get_name(doc, encoding, b"BaseEncoding").map_or_else(
                    || Vec::from(PDFDocEncoding),
                    |base_encoding| {
                        dlog!("BaseEncoding {:?}", base_encoding);
                        encoding_to_unicode_table(base_encoding)
                    },
                );
                let differences = maybe_get_array(doc, encoding, b"Differences");
                if let Some(differences) = differences {
                    dlog!("Differences");
                    let mut code = 0;
                    for o in differences {
                        let o = maybe_deref(doc, o);
                        match o {
                            Object::Integer(i) => {
                                code = *i;
                            }
                            Object::Name(ref n) => {
                                let name = pdf_to_utf8(n);
                                // XXX: names of Type1 fonts can map to arbitrary strings instead of real
                                // unicode names, so we should probably handle this differently
                                let unicode = glyphnames::name_to_unicode(&name);
                                if let Some(unicode) = unicode {
                                    table[code as usize] = unicode;
                                    if let Some(ref mut unicode_map) = unicode_map {
                                        let be = [unicode];
                                        match unicode_map.entry(code as u32) {
                                            // If there's a unicode table entry missing use one based on the name
                                            Entry::Vacant(v) => {
                                                v.insert(String::from_utf16(&be).unwrap());
                                            }
                                            Entry::Occupied(e) => {
                                                if e.get() != &String::from_utf16(&be).unwrap() {
                                                    let normal_match =
                                                        e.get().nfkc().eq(String::from_utf16(&be)
                                                            .unwrap()
                                                            .nfkc());
                                                    println!(
                                                        "Unicode mismatch {} {} {:?} {:?} {:?}",
                                                        normal_match,
                                                        name,
                                                        e.get(),
                                                        String::from_utf16(&be),
                                                        be
                                                    );
                                                }
                                            }
                                        }
                                    }
                                } else {
                                    match unicode_map {
                                        Some(ref mut unicode_map)
                                            if base_name.contains("FontAwesome") =>
                                        {
                                            // the fontawesome tex package will use glyph names that don't have a corresponding unicode
                                            // code point, so we'll use an empty string instead. See issue #76
                                            match unicode_map.entry(code as u32) {
                                                Entry::Vacant(v) => {
                                                    v.insert(String::new());
                                                }
                                                Entry::Occupied(_) => {
                                                    panic!("unexpected entry in unicode map")
                                                }
                                            }
                                        }
                                        _ => {
                                            println!(
                                                "unknown glyph name '{name}' for font {base_name}"
                                            );
                                        }
                                    }
                                }
                                dlog!("{} = {} ({:?})", code, name, unicode);
                                if let Some(ref mut unicode_map) = unicode_map {
                                    // The unicode map might not have the code in it, but the code might
                                    // not be used so we don't want to panic here.
                                    // An example of this is the 'suppress' character in the TeX Latin Modern font.
                                    // This shows up in https://arxiv.org/pdf/2405.01295v1.pdf
                                    dlog!("{} {:?}", code, unicode_map.get(&(code as u32)));
                                }
                                code += 1;
                            }
                            _ => {
                                panic!("wrong type {o:?}");
                            }
                        }
                    }
                }
                // "Type" is optional
                let name = encoding
                    .get(b"Type")
                    .and_then(|x| x.as_name())
                    .map(pdf_to_utf8);
                dlog!("name: {}", name);

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
                    encoding_table = Some(table);
                } else if subtype == "TrueType" {
                    encoding_table = Some(
                        encodings::WIN_ANSI_ENCODING
                            .iter()
                            .map(|x| {
                                if let &Some(x) = x {
                                    glyphnames::name_to_unicode(x).unwrap()
                                } else {
                                    0
                                }
                            })
                            .collect(),
                    );
                }
            }
            _ => {
                panic!();
            }
        }

        let mut width_map = HashMap::new();
        /* "Ordinarily, a font dictionary that refers to one of the standard fonts
        should omit the FirstChar, LastChar, Widths, and FontDescriptor entries.
        However, it is permissible to override a standard font by including these
        entries and embedding the font program in the PDF file."

        Note: some PDFs include a descriptor but still don't include these entries */

        // If we have widths prefer them over the core font widths. Needed for https://dkp.de/wp-content/uploads/parteitage/Sozialismusvorstellungen-der-DKP.pdf
        if let (Some(first_char), Some(last_char), Some(widths)) = (
            maybe_get::<i64>(doc, font, b"FirstChar"),
            maybe_get::<i64>(doc, font, b"LastChar"),
            maybe_get::<Vec<f64>>(doc, font, b"Widths"),
        ) {
            // Some PDF's don't have these like fips-197.pdf
            let mut i: i64 = 0;
            dlog!(
                "first_char {:?}, last_char: {:?}, widths: {} {:?}",
                first_char,
                last_char,
                widths.len(),
                widths
            );

            for w in widths {
                width_map.insert((first_char + i) as CharCode, w);
                i += 1;
            }
            assert_eq!(first_char + i - 1, last_char);
        } else {
            let _name = if is_core_font(&base_name) {
                &base_name
            } else {
                println!("no widths and not core font {base_name:?}");

                // This situation is handled differently by different readers
                // but basically we try to substitute the best font that we can.

                // Poppler/Xpdf:
                // this is technically an error -- the Widths entry is required
                // for all but the Base-14 fonts -- but certain PDF generators
                // apparently don't include widths for Arial and TimesNewRoman

                // Pdfium: CFX_FontMapper::FindSubstFont

                // mupdf: pdf_load_substitute_font

                // We can try to do a better job guessing at a font by looking at the flags
                // or the basename but for now we'll just use Helvetica
                "Helvetica"
            };
            for font_metrics in &core_fonts::metrics() {
                if font_metrics.0 == base_name {
                    if let Some(ref encoding) = encoding_table {
                        dlog!("has encoding");
                        for w in font_metrics.2 {
                            let c = glyphnames::name_to_unicode(w.2).unwrap();
                            (0..encoding.len()).for_each(|i| {
                                if encoding[i] == c {
                                    width_map.insert(i as CharCode, w.1);
                                }
                            });
                        }
                    } else {
                        // Instead of using the encoding from the core font we'll just look up all
                        // of the character names. We should probably verify that this produces the
                        // same result.

                        let mut table = vec![0; 256];
                        for w in font_metrics.2 {
                            dlog!("{} {}", w.0, w.2);
                            // -1 is "not encoded"
                            if w.0 != -1 {
                                table[w.0 as usize] = if base_name == "ZapfDingbats" {
                                    zapfglyphnames::zapfdigbats_names_to_unicode(w.2)
                                        .unwrap_or_else(|| panic!("bad name {w:?}"))
                                } else {
                                    glyphnames::name_to_unicode(w.2).unwrap()
                                }
                            }
                        }

                        let encoding = &table[..];
                        for w in font_metrics.2 {
                            width_map.insert(w.0 as CharCode, w.1);
                            // -1 is "not encoded"
                        }
                        encoding_table = Some(encoding.to_vec());
                    }
                    /* "Ordinarily, a font dictionary that refers to one of the standard fonts
                    should omit the FirstChar, LastChar, Widths, and FontDescriptor entries.
                    However, it is permissible to override a standard font by including these
                    entries and embedding the font program in the PDF file."

                    Note: some PDFs include a descriptor but still don't include these entries */
                    // assert!(maybe_get_obj(doc, font, b"FirstChar").is_none());
                    // assert!(maybe_get_obj(doc, font, b"LastChar").is_none());
                    // assert!(maybe_get_obj(doc, font, b"Widths").is_none());
                }
            }
        }

        let missing_width = get::<Option<f64>>(doc, font, b"MissingWidth").unwrap_or(0.);
        PdfSimpleFont {
            doc,
            font,
            widths: width_map,
            encoding: encoding_table,
            missing_width,
            unicode_map,
        }
    }

    #[allow(dead_code)]
    fn get_type(&self) -> String {
        get_name_string(self.doc, self.font, b"Type")
    }
    #[allow(dead_code)]
    fn get_basefont(&self) -> String {
        get_name_string(self.doc, self.font, b"BaseFont")
    }
    #[allow(dead_code)]
    fn get_subtype(&self) -> String {
        get_name_string(self.doc, self.font, b"Subtype")
    }
    #[allow(dead_code)]
    fn get_widths(&self) -> Option<&Vec<Object>> {
        maybe_get_obj(self.doc, self.font, b"Widths")
            .map(|widths| widths.as_array().expect("Widths should be an array"))
    }
    /* For type1: This entry is obsolescent and its use is no longer recommended. (See
     * implementation note 42 in Appendix H.) */
    #[allow(dead_code)]
    fn get_name(&self) -> Option<String> {
        maybe_get_name_string(self.doc, self.font, b"Name")
    }

    #[allow(dead_code)]
    fn get_descriptor(&self) -> Option<PdfFontDescriptor> {
        maybe_get_obj(self.doc, self.font, b"FontDescriptor")
            .and_then(|desc| desc.as_dict().ok())
            .map(|desc| PdfFontDescriptor {
                desc,
                doc: self.doc,
            })
    }
}

impl<'a> PdfType3Font<'a> {
    fn new(doc: &'a Document, font: &'a Dictionary) -> Self {
        let unicode_map = get_unicode_map(doc, font);
        let encoding: Option<&Object> = get(doc, font, b"Encoding");

        let encoding_table;
        match encoding {
            Some(Object::Name(encoding_name)) => {
                dlog!("encoding {:?}", pdf_to_utf8(encoding_name));
                encoding_table = Some(encoding_to_unicode_table(encoding_name));
            }
            Some(Object::Dictionary(encoding)) => {
                //dlog!("Encoding {:?}", encoding);
                let mut table = maybe_get_name(doc, encoding, b"BaseEncoding").map_or_else(
                    || Vec::from(PDFDocEncoding),
                    |base_encoding| {
                        dlog!("BaseEncoding {:?}", base_encoding);
                        encoding_to_unicode_table(base_encoding)
                    },
                );
                let differences = maybe_get_array(doc, encoding, b"Differences");
                if let Some(differences) = differences {
                    dlog!("Differences");
                    let mut code = 0;
                    for o in differences {
                        match *o {
                            Object::Integer(i) => {
                                code = i;
                            }
                            Object::Name(ref n) => {
                                let name = pdf_to_utf8(n);
                                // XXX: names of Type1 fonts can map to arbitrary strings instead of real
                                // unicode names, so we should probably handle this differently
                                let unicode = glyphnames::name_to_unicode(&name);
                                if let Some(unicode) = unicode {
                                    table[code as usize] = unicode;
                                }
                                dlog!("{} = {} ({:?})", code, name, unicode);
                                if let Some(ref unicode_map) = unicode_map {
                                    dlog!("{} {:?}", code, unicode_map.get(&(code as u32)));
                                }
                                code += 1;
                            }
                            _ => {
                                panic!("wrong type");
                            }
                        }
                    }
                }
                let name_encoded = encoding.get(b"Type");
                if let Ok(Object::Name(name)) = name_encoded {
                    dlog!("name: {}", pdf_to_utf8(name));
                } else {
                    dlog!("name not found");
                }

                encoding_table = Some(table);
            }
            _ => {
                panic!();
            }
        }

        let first_char: i64 = get(doc, font, b"FirstChar");
        let last_char: i64 = get(doc, font, b"LastChar");
        let widths: Vec<f64> = get(doc, font, b"Widths");

        let mut width_map = HashMap::new();

        let mut i = 0;
        dlog!(
            "first_char {:?}, last_char: {:?}, widths: {} {:?}",
            first_char,
            last_char,
            widths.len(),
            widths
        );

        for w in widths {
            width_map.insert((first_char + i) as CharCode, w);
            i += 1;
        }
        assert_eq!(first_char + i - 1, last_char);
        PdfType3Font {
            doc,
            font,
            widths: width_map,
            encoding: encoding_table,
            unicode_map,
        }
    }
}

type CharCode = u32;

struct PdfFontIter<'a> {
    i: Iter<'a, u8>,
    font: &'a dyn PdfFont,
}

impl Iterator for PdfFontIter<'_> {
    type Item = (CharCode, u8);
    fn next(&mut self) -> Option<(CharCode, u8)> {
        self.font.next_char(&mut self.i)
    }
}

trait PdfFont: Debug {
    fn get_width(&self, id: CharCode) -> f64;
    fn next_char(&self, iter: &mut Iter<u8>) -> Option<(CharCode, u8)>;
    fn decode_char(&self, char: CharCode) -> String;

    /*fn char_codes<'a>(&'a self, chars: &'a [u8]) -> PdfFontIter {
        let p = self;
        PdfFontIter{i: chars.iter(), font: p as &PdfFont}
    }*/
}

impl<'a> dyn PdfFont + 'a {
    fn char_codes(&'a self, chars: &'a [u8]) -> PdfFontIter<'a> {
        PdfFontIter {
            i: chars.iter(),
            font: self,
        }
    }
    fn decode(&self, chars: &[u8]) -> String {
        let strings = self
            .char_codes(chars)
            .map(|x| self.decode_char(x.0))
            .collect::<Vec<_>>();
        strings.join("")
    }
}

impl PdfFont for PdfSimpleFont<'_> {
    fn get_width(&self, id: CharCode) -> f64 {
        let width = self.widths.get(&id);
        width.map_or_else(
            || {
                let mut widths = self.widths.iter().collect::<Vec<_>>();
                widths.sort_by_key(|x| x.0);
                dlog!(
                 "missing width for {} len(widths) = {}, {:?} falling back to missing_width {:?}",
                 id,
                 self.widths.len(),
                 widths,
                 self.font
             );
                self.missing_width
            },
            |width| *width,
        )
    }
    /*fn decode(&self, chars: &[u8]) -> String {
        let encoding = self.encoding.as_ref().map(|x| &x[..]).unwrap_or(&PDFDocEncoding);
        to_utf8(encoding, chars)
    }*/

    fn next_char(&self, iter: &mut Iter<u8>) -> Option<(CharCode, u8)> {
        iter.next().map(|x| (CharCode::from(*x), 1))
    }
    fn decode_char(&self, char: CharCode) -> String {
        let slice = [char as u8];
        if let Some(ref unicode_map) = self.unicode_map {
            let s = unicode_map.get(&char);
            let s = s.map_or_else(
                || {
                    println!(
                        "missing char {:?} in unicode map {:?} for {:?}",
                        char, unicode_map, self.font
                    );
                    // some pdf's like http://arxiv.org/pdf/2312.00064v1 are missing entries in their unicode map but do have
                    // entries in the encoding.
                    let encoding = self
                        .encoding
                        .as_ref()
                        .map(|x| &x[..])
                        .expect("missing unicode map and encoding");
                    let s = to_utf8(encoding, &slice);
                    println!("falling back to encoding {char} -> {s:?}");
                    s
                },
                std::clone::Clone::clone,
            );
            return s;
        }
        let encoding = self.encoding.as_ref().map_or(PDFDocEncoding, |x| &x[..]);
        //dlog!("char_code {:?} {:?}", char, self.encoding);

        to_utf8(encoding, &slice)
    }
}

impl fmt::Debug for PdfSimpleFont<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.font.fmt(f)
    }
}

impl PdfFont for PdfType3Font<'_> {
    fn get_width(&self, id: CharCode) -> f64 {
        let width = self.widths.get(&id);
        width.map_or_else(
            || {
                panic!("missing width for {} {:?}", id, self.font);
            },
            |width| *width,
        )
    }
    /*fn decode(&self, chars: &[u8]) -> String {
        let encoding = self.encoding.as_ref().map(|x| &x[..]).unwrap_or(&PDFDocEncoding);
        to_utf8(encoding, chars)
    }*/

    fn next_char(&self, iter: &mut Iter<u8>) -> Option<(CharCode, u8)> {
        iter.next().map(|x| (CharCode::from(*x), 1))
    }
    fn decode_char(&self, char: CharCode) -> String {
        let slice = [char as u8];
        if let Some(ref unicode_map) = self.unicode_map {
            let s = unicode_map.get(&char);
            let s = s.map_or_else(
                || {
                    println!(
                        "missing char {:?} in unicode map {:?} for {:?}",
                        char, unicode_map, self.font
                    );
                    // some pdf's like http://arxiv.org/pdf/2312.00577v1 are missing entries in their unicode map but do have
                    // entries in the encoding.
                    let encoding = self
                        .encoding
                        .as_ref()
                        .map(|x| &x[..])
                        .expect("missing unicode map and encoding");
                    let s = to_utf8(encoding, &slice);
                    println!("falling back to encoding {char} -> {s:?}");
                    s
                },
                std::clone::Clone::clone,
            );

            return s;
        }
        let encoding = self.encoding.as_ref().map_or(PDFDocEncoding, |x| &x[..]);
        //dlog!("char_code {:?} {:?}", char, self.encoding);

        to_utf8(encoding, &slice)
    }
}

impl fmt::Debug for PdfType3Font<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.font.fmt(f)
    }
}

struct PdfCIDFont<'a> {
    font: &'a Dictionary,
    #[allow(dead_code)]
    doc: &'a Document,
    #[allow(dead_code)]
    encoding: ByteMapping,
    to_unicode: Option<HashMap<u32, String>>,
    widths: HashMap<CharCode, f64>, // should probably just use i32 here
    default_width: Option<f64>, // only used for CID fonts and we should probably brake out the different font types
}

fn get_unicode_map<'a>(doc: &'a Document, font: &'a Dictionary) -> Option<HashMap<u32, String>> {
    let to_unicode = maybe_get_obj(doc, font, b"ToUnicode");
    dlog!("ToUnicode: {:?}", to_unicode);
    let mut unicode_map = None;
    match to_unicode {
        Some(Object::Stream(stream)) => {
            let contents = get_contents(stream);
            dlog!("Stream: {}", String::from_utf8(contents.clone()).unwrap());

            let cmap = adobe_cmap_parser::get_unicode_map(&contents).unwrap();
            let mut unicode = HashMap::new();
            // "It must use the beginbfchar, endbfchar, beginbfrange, and endbfrange operators to
            // define the mapping from character codes to Unicode character sequences expressed in
            // UTF-16BE encoding."
            for (&k, v) in &cmap {
                let mut be: Vec<u16> = Vec::new();
                let mut i = 0;
                assert!(v.len() % 2 == 0);
                while i < v.len() {
                    be.push((u16::from(v[i]) << 8) | u16::from(v[i + 1]));
                    i += 2;
                }
                if let [0xd800..=0xdfff] = &be[..] {
                    // this range is not specified as not being encoded
                    // we ignore them so we don't an error from from_utt16
                    continue;
                }
                let s = String::from_utf16(&be).unwrap();

                unicode.insert(k, s);
            }
            unicode_map = Some(unicode);

            dlog!("map: {:?}", unicode_map);
        }
        None => {}
        Some(Object::Name(name)) => {
            let name = pdf_to_utf8(name);
            if name != "Identity-H" {
                todo!("unsupported ToUnicode name: {:?}", name);
            }
        }
        _ => {
            panic!("unsupported cmap {to_unicode:?}")
        }
    }
    unicode_map
}

impl<'a> PdfCIDFont<'a> {
    fn new(doc: &'a Document, font: &'a Dictionary) -> Self {
        let base_name = get_name_string(doc, font, b"BaseFont");
        let descendants =
            maybe_get_array(doc, font, b"DescendantFonts").expect("Descendant fonts required");
        let ciddict = maybe_deref(doc, &descendants[0])
            .as_dict()
            .expect("should be CID dict");
        let encoding =
            maybe_get_obj(doc, font, b"Encoding").expect("Encoding required in type0 fonts");
        dlog!("base_name {} {:?}", base_name, font);

        let encoding = match encoding {
            Object::Name(name) => {
                let name = pdf_to_utf8(name);
                dlog!("encoding {:?}", name);
                if name == "Identity-H" || name == "Identity-V" {
                    ByteMapping {
                        codespace: vec![CodeRange {
                            width: 2,
                            start: 0,
                            end: 0xffff,
                        }],
                        cid: vec![CIDRange {
                            src_code_lo: 0,
                            src_code_hi: 0xffff,
                            dst_CID_lo: 0,
                        }],
                    }
                } else {
                    panic!("unsupported encoding {name}");
                }
            }
            Object::Stream(stream) => {
                let contents = get_contents(stream);
                dlog!("Stream: {}", String::from_utf8(contents.clone()).unwrap());
                adobe_cmap_parser::get_byte_mapping(&contents).unwrap()
            }
            _ => {
                panic!("unsupported encoding {encoding:?}")
            }
        };

        // Sometimes a Type0 font might refer to the same underlying data as regular font. In this case we may be able to extract some encoding
        // data.
        // We should also look inside the truetype data to see if there's a cmap table. It will help us convert as well.
        // This won't work if the cmap has been subsetted. A better approach might be to hash glyph contents and use that against
        // a global library of glyph hashes
        let unicode_map = get_unicode_map(doc, font);

        dlog!("descendents {:?} {:?}", descendants, ciddict);

        let font_dict = maybe_get_obj(doc, ciddict, b"FontDescriptor").expect("required");
        dlog!("{:?}", font_dict);
        let _f = font_dict.as_dict().expect("must be dict");
        let default_width = get::<Option<i64>>(doc, ciddict, b"DW").unwrap_or(1000);
        let w: Option<Vec<&Object>> = get(doc, ciddict, b"W");
        dlog!("widths {:?}", w);
        let mut widths = HashMap::new();
        let mut i = 0;
        if let Some(w) = w {
            while i < w.len() {
                if let Object::Array(wa) = w[i + 1] {
                    let cid = w[i].as_i64().expect("id should be num");
                    dlog!("wa: {:?} -> {:?}", cid, wa);
                    for (j, w) in wa.iter().enumerate() {
                        widths.insert((cid + j as i64) as CharCode, as_num(w));
                    }
                    i += 2;
                } else {
                    let c_first = w[i].as_i64().expect("first should be num");
                    let c_last = w[i].as_i64().expect("last should be num");
                    let c_width = as_num(w[i]);
                    for id in c_first..c_last {
                        widths.insert(id as CharCode, c_width);
                    }
                    i += 3;
                }
            }
        }
        PdfCIDFont {
            doc,
            font,
            widths,
            to_unicode: unicode_map,
            encoding,
            default_width: Some(default_width as f64),
        }
    }
}

impl PdfFont for PdfCIDFont<'_> {
    fn get_width(&self, id: CharCode) -> f64 {
        let width = self.widths.get(&id);
        width.map_or_else(
            || {
                dlog!("missing width for {} falling back to default_width", id);
                self.default_width.unwrap()
            },
            |width| {
                dlog!("GetWidth {} -> {}", id, *width);
                *width
            },
        )
    } /*
      fn decode(&self, chars: &[u8]) -> String {
          self.char_codes(chars);

          //let utf16 = Vec::new();

          let encoding = self.encoding.as_ref().map(|x| &x[..]).unwrap_or(&PDFDocEncoding);
          to_utf8(encoding, chars)
      }*/

    fn next_char(&self, iter: &mut Iter<u8>) -> Option<(CharCode, u8)> {
        let mut c = u32::from(*iter.next()?);
        let mut code = None;
        'outer: for width in 1..=4 {
            for range in &self.encoding.codespace {
                if c >= range.start && c <= range.end && range.width == width {
                    code = Some((c, width));
                    break 'outer;
                }
            }
            let next = *iter.next()?;
            c = (c << 8) | u32::from(next);
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
        s.map_or_else(
            || {
                dlog!(
                    "Unknown character {:?} in {:?} {:?}",
                    char,
                    self.font,
                    self.to_unicode
                );
                String::new()
            },
            std::clone::Clone::clone,
        )
    }
}

impl fmt::Debug for PdfCIDFont<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.font.fmt(f)
    }
}

#[derive(Copy, Clone)]
struct PdfFontDescriptor<'a> {
    desc: &'a Dictionary,
    doc: &'a Document,
}

impl<'a> PdfFontDescriptor<'a> {
    #[allow(dead_code)]
    fn get_file(&self) -> Option<&'a Object> {
        maybe_get_obj(self.doc, self.desc, b"FontFile")
    }
}

impl fmt::Debug for PdfFontDescriptor<'_> {
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
    if divisor == 0. {
        // (x - x_min) will be 0 which means we want to discard the interpolation
        // and arbitrarily choose y_min to match pdfium
        y_min
    } else {
        (x - x_min).mul_add((y_max - y_min) / divisor, y_min)
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
    Type4,
}

impl Function {
    fn new(doc: &Document, obj: &Object) -> Self {
        let dict = match obj {
            Object::Dictionary(dict) => dict,
            Object::Stream(stream) => &stream.dict,
            _ => panic!(),
        };
        let function_type: i64 = get(doc, dict, b"FunctionType");

        match function_type {
            0 => {
                let stream = if let Object::Stream(stream) = obj {
                    stream
                } else {
                    panic!()
                };
                let range: Vec<f64> = get(doc, dict, b"Range");
                let domain: Vec<f64> = get(doc, dict, b"Domain");
                let contents = get_contents(stream);
                let size: Vec<i64> = get(doc, dict, b"Size");
                let bits_per_sample = get(doc, dict, b"BitsPerSample");
                // We ignore 'Order' like pdfium, poppler and pdf.js

                let encode = get::<Option<Vec<f64>>>(doc, dict, b"Encode");
                // maybe there's some better way to write this.
                let encode = encode.unwrap_or_else(|| {
                    let mut default = Vec::new();
                    for i in &size {
                        default.extend([0., (i - 1) as f64].iter());
                    }
                    default
                });
                let decode =
                    get::<Option<Vec<f64>>>(doc, dict, b"Decode").unwrap_or_else(|| range.clone());

                Self::Type0(Type0Func {
                    domain,
                    range,
                    contents,
                    size,
                    bits_per_sample,
                    encode,
                    decode,
                })
            }
            2 => {
                let c0 = get::<Option<Vec<f64>>>(doc, dict, b"C0");
                let c1 = get::<Option<Vec<f64>>>(doc, dict, b"C1");
                let n = get::<f64>(doc, dict, b"N");
                Self::Type2(Type2Func { c0, c1, n })
            }
            _ => {
                panic!("unhandled function type {function_type}")
            }
        }
    }
}

fn as_num(o: &Object) -> f64 {
    match *o {
        Object::Integer(i) => i as f64,
        Object::Real(f) => f.into(),
        _ => {
            panic!("not a number")
        }
    }
}

#[derive(Clone)]
struct TextState<'a> {
    font: Option<Rc<dyn PdfFont + 'a>>,
    font_size: f64,
    character_spacing: f64,
    word_spacing: f64,
    horizontal_scaling: f64,
    leading: f64,
    rise: f64,
    tm: Transform,
}

// XXX: We'd ideally implement this without having to copy the uncompressed data
fn get_contents(contents: &Stream) -> Vec<u8> {
    if contents.filter().is_ok() {
        contents
            .decompressed_content()
            .unwrap_or_else(|_| contents.content.clone())
    } else {
        contents.content.clone()
    }
}

#[derive(Clone)]
struct GraphicsState<'a> {
    ctm: Transform,
    ts: TextState<'a>,
    smask: Option<Dictionary>,
    fill_colorspace: ColorSpace,
    fill_color: Vec<f64>,
    stroke_colorspace: ColorSpace,
    stroke_color: Vec<f64>,
    line_width: f64,
}

fn show_text(
    gs: &mut GraphicsState,
    s: &[u8],
    _tlm: &Transform,
    _flip_ctm: &Transform,
    output: &mut dyn OutputDev,
) -> Result<(), OutputError> {
    let ts = &mut gs.ts;
    let font = ts.font.as_ref().unwrap();
    //let encoding = font.encoding.as_ref().map(|x| &x[..]).unwrap_or(&PDFDocEncoding);
    dlog!("{:?}", font.decode(s));
    dlog!("{:?}", font.decode(s).as_bytes());
    dlog!("{:?}", s);
    output.begin_word()?;

    for (c, length) in font.char_codes(s) {
        // 5.3.3 Text Space Details
        let tsm = Transform2D::row_major(ts.horizontal_scaling, 0., 0., 1.0, 0., ts.rise);
        // Trm = Tsm × Tm × CTM
        let trm = tsm.post_transform(&ts.tm.post_transform(&gs.ctm));
        //dlog!("ctm: {:?} tm {:?}", gs.ctm, tm);
        //dlog!("current pos: {:?}", position);
        // 5.9 Extraction of Text Content

        //dlog!("w: {}", font.widths[&(*c as i64)]);
        let w0 = font.get_width(c) / 1000.;

        let mut spacing = ts.character_spacing;
        // "Word spacing is applied to every occurrence of the single-byte character code 32 in a
        //  string when using a simple font or a composite font that defines code 32 as a
        //  single-byte code. It does not apply to occurrences of the byte value 32 in
        //  multiple-byte codes."
        let is_space = c == 32 && length == 1;
        if is_space {
            spacing += ts.word_spacing;
        }

        output.output_character(&trm, w0, spacing, ts.font_size, &font.decode_char(c))?;
        let tj = 0.;
        let ty = 0.;
        let tx = ts.horizontal_scaling * (w0 - tj / 1000.).mul_add(ts.font_size, spacing);
        dlog!(
            "horizontal {} adjust {} {} {} {}",
            ts.horizontal_scaling,
            tx,
            w0,
            ts.font_size,
            spacing
        );
        // dlog!("w0: {}, tx: {}", w0, tx);
        ts.tm = ts
            .tm
            .pre_transform(&Transform2D::create_translation(tx, ty));
        let _trm = ts.tm.pre_transform(&gs.ctm);
        //dlog!("post pos: {:?}", trm);
    }
    output.end_word()?;
    Ok(())
}

#[derive(Debug, Clone, Copy)]
pub struct MediaBox {
    pub llx: f64,
    pub lly: f64,
    pub urx: f64,
    pub ury: f64,
}

fn apply_state(doc: &Document, gs: &mut GraphicsState, state: &Dictionary) {
    for (k, v) in state {
        let k: &[u8] = k.as_ref();
        match k {
            b"SMask" => match *maybe_deref(doc, v) {
                Object::Name(ref name) => {
                    if name == b"None" {
                        gs.smask = None;
                    } else {
                        panic!("unexpected smask name")
                    }
                }
                Object::Dictionary(ref dict) => {
                    gs.smask = Some(dict.clone());
                }
                _ => {
                    panic!("unexpected smask type {v:?}")
                }
            },
            b"Type" => match v {
                Object::Name(name) => {
                    assert_eq!(name, b"ExtGState");
                }
                _ => {
                    panic!("unexpected type")
                }
            },
            _ => {
                dlog!("unapplied state: {:?} {:?}", k, v);
            }
        }
    }
}

#[derive(Debug)]
pub enum PathOp {
    MoveTo(f64, f64),
    LineTo(f64, f64),
    // XXX: is it worth distinguishing the different kinds of curve ops?
    CurveTo(f64, f64, f64, f64, f64, f64),
    Rect(f64, f64, f64, f64),
    Close,
}

#[derive(Debug)]
pub struct Path {
    pub ops: Vec<PathOp>,
}

impl Path {
    fn new() -> Self {
        Self { ops: Vec::new() }
    }
    fn current_point(&self) -> (f64, f64) {
        match *self.ops.last().unwrap() {
            PathOp::MoveTo(x, y) => (x, y),
            PathOp::LineTo(x, y) => (x, y),
            PathOp::CurveTo(_, _, _, _, x, y) => (x, y),
            _ => {
                panic!()
            }
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
    matrix: Option<Vec<f64>>,
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
    ICCBased(Vec<u8>),
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
    ICCBased(Vec<u8>),
}

fn make_colorspace<'a>(doc: &'a Document, name: &[u8], resources: &'a Dictionary) -> ColorSpace {
    match name {
        b"DeviceGray" => ColorSpace::DeviceGray,
        b"DeviceRGB" => ColorSpace::DeviceRGB,
        b"DeviceCMYK" => ColorSpace::DeviceCMYK,
        b"Pattern" => ColorSpace::Pattern,
        _ => {
            let colorspaces: &Dictionary = get(doc, resources, b"ColorSpace");
            let cs: &Object = maybe_get_obj(doc, colorspaces, name)
                .unwrap_or_else(|| panic!("missing colorspace {name:?}"));

            cs.as_array().map_or_else(
                |_| {
                    cs.as_name().map_or_else(
                        |_| {
                            panic!();
                        },
                        |cs| match pdf_to_utf8(cs).as_ref() {
                            "DeviceRGB" => ColorSpace::DeviceRGB,
                            "DeviceGray" => ColorSpace::DeviceGray,
                            _ => panic!(),
                        },
                    )
                },
                |cs| {
                    let cs_name = pdf_to_utf8(cs[0].as_name().expect("first arg must be a name"));
                    match cs_name.as_ref() {
                        "Separation" => {
                            let name =
                                pdf_to_utf8(cs[1].as_name().expect("second arg must be a name"));
                            let alternate_space = match &maybe_deref(doc, &cs[2]) {
                                Object::Name(name) => match &name[..] {
                                    b"DeviceGray" => AlternateColorSpace::DeviceGray,
                                    b"DeviceRGB" => AlternateColorSpace::DeviceRGB,
                                    b"DeviceCMYK" => AlternateColorSpace::DeviceCMYK,
                                    _ => panic!("unexpected color space name"),
                                },
                                Object::Array(cs) => {
                                    let cs_name = pdf_to_utf8(
                                        cs[0].as_name().expect("first arg must be a name"),
                                    );
                                    match cs_name.as_ref() {
                                        "ICCBased" => {
                                            let stream =
                                                maybe_deref(doc, &cs[1]).as_stream().unwrap();
                                            dlog!("ICCBased {:?}", stream);
                                            // XXX: we're going to be continually decompressing everytime this object is referenced
                                            AlternateColorSpace::ICCBased(get_contents(stream))
                                        }
                                        "CalGray" => {
                                            let dict =
                                                cs[1].as_dict().expect("second arg must be a dict");
                                            AlternateColorSpace::CalGray(CalGray {
                                                white_point: get(doc, dict, b"WhitePoint"),
                                                black_point: get(doc, dict, b"BackPoint"),
                                                gamma: get(doc, dict, b"Gamma"),
                                            })
                                        }
                                        "CalRGB" => {
                                            let dict =
                                                cs[1].as_dict().expect("second arg must be a dict");
                                            AlternateColorSpace::CalRGB(CalRGB {
                                                white_point: get(doc, dict, b"WhitePoint"),
                                                black_point: get(doc, dict, b"BackPoint"),
                                                gamma: get(doc, dict, b"Gamma"),
                                                matrix: get(doc, dict, b"Matrix"),
                                            })
                                        }
                                        "Lab" => {
                                            let dict =
                                                cs[1].as_dict().expect("second arg must be a dict");
                                            AlternateColorSpace::Lab(Lab {
                                                white_point: get(doc, dict, b"WhitePoint"),
                                                black_point: get(doc, dict, b"BackPoint"),
                                                range: get(doc, dict, b"Range"),
                                            })
                                        }
                                        _ => panic!("Unexpected color space name"),
                                    }
                                }
                                _ => panic!("Alternate space should be name or array {:?}", cs[2]),
                            };
                            let tint_transform =
                                Box::new(Function::new(doc, maybe_deref(doc, &cs[3])));

                            dlog!("{:?} {:?} {:?}", name, alternate_space, tint_transform);
                            ColorSpace::Separation(Separation {
                                name,
                                alternate_space,
                                tint_transform,
                            })
                        }
                        "ICCBased" => {
                            let stream = maybe_deref(doc, &cs[1]).as_stream().unwrap();
                            dlog!("ICCBased {:?}", stream);
                            // XXX: we're going to be continually decompressing everytime this object is referenced
                            ColorSpace::ICCBased(get_contents(stream))
                        }
                        "CalGray" => {
                            let dict = cs[1].as_dict().expect("second arg must be a dict");
                            ColorSpace::CalGray(CalGray {
                                white_point: get(doc, dict, b"WhitePoint"),
                                black_point: get(doc, dict, b"BackPoint"),
                                gamma: get(doc, dict, b"Gamma"),
                            })
                        }
                        "CalRGB" => {
                            let dict = cs[1].as_dict().expect("second arg must be a dict");
                            ColorSpace::CalRGB(CalRGB {
                                white_point: get(doc, dict, b"WhitePoint"),
                                black_point: get(doc, dict, b"BackPoint"),
                                gamma: get(doc, dict, b"Gamma"),
                                matrix: get(doc, dict, b"Matrix"),
                            })
                        }
                        "Lab" => {
                            let dict = cs[1].as_dict().expect("second arg must be a dict");
                            ColorSpace::Lab(Lab {
                                white_point: get(doc, dict, b"WhitePoint"),
                                black_point: get(doc, dict, b"BackPoint"),
                                range: get(doc, dict, b"Range"),
                            })
                        }
                        "Pattern" => ColorSpace::Pattern,
                        "DeviceGray" => ColorSpace::DeviceGray,
                        "DeviceRGB" => ColorSpace::DeviceRGB,
                        "DeviceCMYK" => ColorSpace::DeviceCMYK,
                        "DeviceN" => ColorSpace::DeviceN,
                        _ => {
                            panic!("color_space {name:?} {cs_name:?} {cs:?}")
                        }
                    }
                },
            )
        }
    }
}

struct Processor<'a> {
    _none: PhantomData<&'a ()>,
}

impl<'a> Processor<'a> {
    fn new() -> Self {
        Processor { _none: PhantomData }
    }

    fn process_stream(
        &mut self,
        doc: &'a Document,
        content: &[u8],
        resources: &'a Dictionary,
        media_box: &MediaBox,
        output: &mut dyn OutputDev,
        page_num: u32,
    ) -> Result<(), OutputError> {
        let content = Content::decode(content).unwrap();
        let mut font_table = HashMap::new();
        let mut gs: GraphicsState = GraphicsState {
            ts: TextState {
                font: None,
                font_size: f64::NAN,
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
            smask: None,
        };
        //let mut ts = &mut gs.ts;
        let mut gs_stack = Vec::new();
        let mut mc_stack = Vec::new();
        // XXX: replace tlm with a point for text start
        let mut tlm = Transform2D::identity();
        let mut path = Path::new();
        let flip_ctm = Transform2D::row_major(1., 0., 0., -1., 0., media_box.ury - media_box.lly);
        dlog!("MediaBox {:?}", media_box);
        for operation in &content.operations {
            //dlog!("op: {:?}", operation);

            match operation.operator.as_ref() {
                "BT" | "ET" => {
                    tlm = Transform2D::identity();
                    gs.ts.tm = tlm;
                }
                "cm" => {
                    assert!(operation.operands.len() == 6);
                    let m = Transform2D::row_major(
                        as_num(&operation.operands[0]),
                        as_num(&operation.operands[1]),
                        as_num(&operation.operands[2]),
                        as_num(&operation.operands[3]),
                        as_num(&operation.operands[4]),
                        as_num(&operation.operands[5]),
                    );
                    gs.ctm = gs.ctm.pre_transform(&m);
                    dlog!("matrix {:?}", gs.ctm);
                }
                "CS" => {
                    let name = operation.operands[0].as_name().unwrap();
                    gs.stroke_colorspace = make_colorspace(doc, name, resources);
                }
                "cs" => {
                    let name = operation.operands[0].as_name().unwrap();
                    gs.fill_colorspace = make_colorspace(doc, name, resources);
                }
                "SC" | "SCN" => {
                    gs.stroke_color = match gs.stroke_colorspace {
                        ColorSpace::Pattern => {
                            dlog!("unhandled pattern color");
                            Vec::new()
                        }
                        _ => operation.operands.iter().map(as_num).collect(),
                    };
                }
                "sc" | "scn" => {
                    gs.fill_color = match gs.fill_colorspace {
                        ColorSpace::Pattern => {
                            dlog!("unhandled pattern color");
                            Vec::new()
                        }
                        _ => operation.operands.iter().map(as_num).collect(),
                    };
                }
                "G" | "g" | "RG" | "rg" | "K" | "k" => {
                    dlog!("unhandled color operation {:?}", operation);
                }
                "TJ" => {
                    if let Object::Array(ref array) = operation.operands[0] {
                        for e in array {
                            match *e {
                                Object::String(ref s, _) => {
                                    show_text(&mut gs, s, &tlm, &flip_ctm, output)?;
                                }
                                Object::Integer(i) => {
                                    let ts = &mut gs.ts;
                                    let w0 = 0.;
                                    let tj = i as f64;
                                    let ty = 0.;
                                    let tx =
                                        ts.horizontal_scaling * ((w0 - tj / 1000.) * ts.font_size);
                                    ts.tm = ts
                                        .tm
                                        .pre_transform(&Transform2D::create_translation(tx, ty));
                                    dlog!("adjust text by: {} {:?}", i, ts.tm);
                                }
                                Object::Real(i) => {
                                    let ts = &mut gs.ts;
                                    let w0 = 0.;
                                    let tj = f64::from(i);
                                    let ty = 0.;
                                    let tx =
                                        ts.horizontal_scaling * ((w0 - tj / 1000.) * ts.font_size);
                                    ts.tm = ts
                                        .tm
                                        .pre_transform(&Transform2D::create_translation(tx, ty));
                                    dlog!("adjust text by: {} {:?}", i, ts.tm);
                                }
                                _ => {
                                    dlog!("kind of {:?}", e);
                                }
                            }
                        }
                    }
                }
                "Tj" => match operation.operands[0] {
                    Object::String(ref s, _) => {
                        show_text(&mut gs, s, &tlm, &flip_ctm, output)?;
                    }
                    _ => {
                        panic!("unexpected Tj operand {operation:?}")
                    }
                },
                "Tc" => {
                    gs.ts.character_spacing = as_num(&operation.operands[0]);
                }
                "Tw" => {
                    gs.ts.word_spacing = as_num(&operation.operands[0]);
                }
                "Tz" => {
                    gs.ts.horizontal_scaling = as_num(&operation.operands[0]) / 100.;
                }
                "TL" => {
                    gs.ts.leading = as_num(&operation.operands[0]);
                }
                "Tf" => {
                    let fonts: &Dictionary = get(doc, resources, b"Font");
                    let name = operation.operands[0].as_name().unwrap();
                    let font = font_table
                        .entry(name.to_owned())
                        .or_insert_with(|| make_font(doc, get::<&Dictionary>(doc, fonts, name)))
                        .clone();
                    {
                        /*let file = font.get_descriptor().and_then(|desc| desc.get_file());
                        if let Some(file) = file {
                            let file_contents = filter_data(file.as_stream().unwrap());
                            let mut cursor = Cursor::new(&file_contents[..]);
                            //let f = Font::read(&mut cursor);
                            //dlog!("font file: {:?}", f);
                        }*/
                    }
                    gs.ts.font = Some(font);

                    gs.ts.font_size = as_num(&operation.operands[1]);
                    dlog!(
                        "font {} size: {} {:?}",
                        pdf_to_utf8(name),
                        gs.ts.font_size,
                        operation
                    );
                }
                "Ts" => {
                    gs.ts.rise = as_num(&operation.operands[0]);
                }
                "Tm" => {
                    assert!(operation.operands.len() == 6);
                    tlm = Transform2D::row_major(
                        as_num(&operation.operands[0]),
                        as_num(&operation.operands[1]),
                        as_num(&operation.operands[2]),
                        as_num(&operation.operands[3]),
                        as_num(&operation.operands[4]),
                        as_num(&operation.operands[5]),
                    );
                    gs.ts.tm = tlm;
                    dlog!("Tm: matrix {:?}", gs.ts.tm);
                    output.end_line()?;
                }
                "Td" => {
                    /* Move to the start of the next line, offset from the start of the current line by (tx , ty ).
                      tx and ty are numbers expressed in unscaled text space units.
                      More precisely, this operator performs the following assignments:
                    */
                    assert!(operation.operands.len() == 2);
                    let tx = as_num(&operation.operands[0]);
                    let ty = as_num(&operation.operands[1]);
                    dlog!("translation: {} {}", tx, ty);

                    tlm = tlm.pre_transform(&Transform2D::create_translation(tx, ty));
                    gs.ts.tm = tlm;
                    dlog!("Td matrix {:?}", gs.ts.tm);
                    output.end_line()?;
                }

                "TD" => {
                    /* Move to the start of the next line, offset from the start of the current line by (tx , ty ).
                      As a side effect, this operator sets the leading parameter in the text state.
                    */
                    assert!(operation.operands.len() == 2);
                    let tx = as_num(&operation.operands[0]);
                    let ty = as_num(&operation.operands[1]);
                    dlog!("translation: {} {}", tx, ty);
                    gs.ts.leading = -ty;

                    tlm = tlm.pre_transform(&Transform2D::create_translation(tx, ty));
                    gs.ts.tm = tlm;
                    dlog!("TD matrix {:?}", gs.ts.tm);
                    output.end_line()?;
                }

                "T*" => {
                    let tx = 0.0;
                    let ty = -gs.ts.leading;

                    tlm = tlm.pre_transform(&Transform2D::create_translation(tx, ty));
                    gs.ts.tm = tlm;
                    dlog!("T* matrix {:?}", gs.ts.tm);
                    output.end_line()?;
                }
                "q" => {
                    gs_stack.push(gs.clone());
                }
                "Q" => {
                    let s = gs_stack.pop();
                    if let Some(s) = s {
                        gs = s;
                    } else {
                        println!("No state to pop");
                    }
                }
                "gs" => {
                    let ext_gstate: &Dictionary = get(doc, resources, b"ExtGState");
                    let name = operation.operands[0].as_name().unwrap();
                    let state: &Dictionary = get(doc, ext_gstate, name);
                    apply_state(doc, &mut gs, state);
                }
                "i" => {
                    dlog!(
                        "unhandled graphics state flattness operator {:?}",
                        operation
                    );
                }
                "w" => {
                    gs.line_width = as_num(&operation.operands[0]);
                }
                "J" | "j" | "M" | "d" | "ri" => {
                    dlog!("unknown graphics state operator {:?}", operation);
                }
                "m" => path.ops.push(PathOp::MoveTo(
                    as_num(&operation.operands[0]),
                    as_num(&operation.operands[1]),
                )),
                "l" => path.ops.push(PathOp::LineTo(
                    as_num(&operation.operands[0]),
                    as_num(&operation.operands[1]),
                )),
                "c" => path.ops.push(PathOp::CurveTo(
                    as_num(&operation.operands[0]),
                    as_num(&operation.operands[1]),
                    as_num(&operation.operands[2]),
                    as_num(&operation.operands[3]),
                    as_num(&operation.operands[4]),
                    as_num(&operation.operands[5]),
                )),
                "v" => {
                    let (x, y) = path.current_point();
                    path.ops.push(PathOp::CurveTo(
                        x,
                        y,
                        as_num(&operation.operands[0]),
                        as_num(&operation.operands[1]),
                        as_num(&operation.operands[2]),
                        as_num(&operation.operands[3]),
                    ));
                }
                "y" => path.ops.push(PathOp::CurveTo(
                    as_num(&operation.operands[0]),
                    as_num(&operation.operands[1]),
                    as_num(&operation.operands[2]),
                    as_num(&operation.operands[3]),
                    as_num(&operation.operands[2]),
                    as_num(&operation.operands[3]),
                )),
                "h" => path.ops.push(PathOp::Close),
                "re" => path.ops.push(PathOp::Rect(
                    as_num(&operation.operands[0]),
                    as_num(&operation.operands[1]),
                    as_num(&operation.operands[2]),
                    as_num(&operation.operands[3]),
                )),
                "s" | "f*" | "B" | "B*" | "b" => {
                    dlog!("unhandled path op {:?}", operation);
                }
                "S" => {
                    output.stroke(&gs.ctm, &gs.stroke_colorspace, &gs.stroke_color, &path)?;
                    path.ops.clear();
                }
                "F" | "f" => {
                    output.fill(&gs.ctm, &gs.fill_colorspace, &gs.fill_color, &path)?;
                    path.ops.clear();
                }
                "W" | "w*" => {
                    dlog!("unhandled clipping operation {:?}", operation);
                }
                "n" => {
                    dlog!("discard {:?}", path);
                    path.ops.clear();
                }
                "BMC" | "BDC" => {
                    mc_stack.push(operation);
                }
                "EMC" => {
                    mc_stack.pop();
                }
                "Do" => {
                    // `Do` process an entire subdocument, so we do a recursive call to `process_stream`
                    // with the subdocument content and resources
                    let xobject: &Dictionary = get(doc, resources, b"XObject");
                    let name = operation.operands[0].as_name().unwrap();
                    let xf: &Stream = get(doc, xobject, name);
                    let resources = maybe_get_obj(doc, &xf.dict, b"Resources")
                        .and_then(|n| n.as_dict().ok())
                        .unwrap_or(resources);
                    let contents = get_contents(xf);
                    self.process_stream(doc, &contents, resources, media_box, output, page_num)?;
                }
                _ => {
                    dlog!("unknown operation {:?}", operation);
                }
            }
        }
        Ok(())
    }
}

pub trait OutputDev {
    fn begin_page(
        &mut self,
        page_num: u32,
        media_box: &MediaBox,
        art_box: Option<(f64, f64, f64, f64)>,
    ) -> Result<(), OutputError>;
    fn end_page(&mut self) -> Result<(), OutputError>;
    fn output_character(
        &mut self,
        trm: &Transform,
        width: f64,
        spacing: f64,
        font_size: f64,
        char: &str,
    ) -> Result<(), OutputError>;
    fn begin_word(&mut self) -> Result<(), OutputError>;
    fn end_word(&mut self) -> Result<(), OutputError>;
    fn end_line(&mut self) -> Result<(), OutputError>;
    fn stroke(
        &mut self,
        _ctm: &Transform,
        _colorspace: &ColorSpace,
        _color: &[f64],
        _path: &Path,
    ) -> Result<(), OutputError> {
        Ok(())
    }
    fn fill(
        &mut self,
        _ctm: &Transform,
        _colorspace: &ColorSpace,
        _color: &[f64],
        _path: &Path,
    ) -> Result<(), OutputError> {
        Ok(())
    }
}

pub struct HTMLOutput<'a> {
    file: &'a mut dyn std::io::Write,
    flip_ctm: Transform,
    last_ctm: Transform,
    buf_ctm: Transform,
    buf_font_size: f64,
    buf: String,
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

impl HTMLOutput<'_> {
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
    fn flush_string(&mut self) -> Result<(), OutputError> {
        if !self.buf.is_empty() {
            let position = self.buf_ctm.post_transform(&self.flip_ctm);
            let transformed_font_size_vec = self
                .buf_ctm
                .transform_vector(vec2(self.buf_font_size, self.buf_font_size));
            // get the length of one sized of the square with the same area with a rectangle of size (x, y)
            let transformed_font_size =
                (transformed_font_size_vec.x * transformed_font_size_vec.y).sqrt();
            let (x, y) = (position.m31, position.m32);
            println!("flush {} {:?}", self.buf, (x, y));

            writeln!(
                self.file,
                "<div style='position: absolute; left: {}px; top: {}px; font-size: {}px'>{}</div>",
                x,
                y,
                transformed_font_size,
                insert_nbsp(&self.buf)
            )?;
        }
        Ok(())
    }
}

type ArtBox = (f64, f64, f64, f64);

impl OutputDev for HTMLOutput<'_> {
    fn begin_page(
        &mut self,
        page_num: u32,
        media_box: &MediaBox,
        _: Option<ArtBox>,
    ) -> Result<(), OutputError> {
        write!(self.file, "<meta charset='utf-8' /> ")?;
        write!(self.file, "<!-- page {page_num} -->")?;
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
    fn output_character(
        &mut self,
        trm: &Transform,
        width: f64,
        spacing: f64,
        font_size: f64,
        char: &str,
    ) -> Result<(), OutputError> {
        if trm.approx_eq(&self.last_ctm) {
            let position = trm.post_transform(&self.flip_ctm);
            let (x, y) = (position.m31, position.m32);

            println!("accum {} {:?}", char, (x, y));
            self.buf += char;
        } else {
            println!(
                "flush {} {:?} {:?} {} {} {}",
                char, trm, self.last_ctm, width, font_size, spacing
            );
            self.flush_string()?;
            char.clone_into(&mut self.buf);
            self.buf_font_size = font_size;
            self.buf_ctm = *trm;
        }
        let position = trm.post_transform(&self.flip_ctm);
        let transformed_font_size_vec = trm.transform_vector(vec2(font_size, font_size));
        // get the length of one sized of the square with the same area with a rectangle of size (x, y)
        let transformed_font_size =
            (transformed_font_size_vec.x * transformed_font_size_vec.y).sqrt();
        let (x, y) = (position.m31, position.m32);
        write!(self.file, "<div style='position: absolute; color: red; left: {x}px; top: {y}px; font-size: {transformed_font_size}px'>{char}</div>")?;
        self.last_ctm = trm.pre_transform(&Transform2D::create_translation(
            width.mul_add(font_size, spacing),
            0.,
        ));

        Ok(())
    }
    fn begin_word(&mut self) -> Result<(), OutputError> {
        Ok(())
    }
    fn end_word(&mut self) -> Result<(), OutputError> {
        Ok(())
    }
    fn end_line(&mut self) -> Result<(), OutputError> {
        Ok(())
    }
}

pub struct SVGOutput<'a> {
    file: &'a mut dyn std::io::Write,
}
impl SVGOutput<'_> {
    pub fn new(file: &mut dyn std::io::Write) -> SVGOutput {
        SVGOutput { file }
    }
}

impl OutputDev for SVGOutput<'_> {
    fn begin_page(
        &mut self,
        _page_num: u32,
        media_box: &MediaBox,
        art_box: Option<(f64, f64, f64, f64)>,
    ) -> Result<(), OutputError> {
        let ver = 1.1;
        writeln!(self.file, "<?xml version=\"1.0\" encoding=\"UTF-8\" ?>")?;
        if ver == 1.1 {
            write!(
                self.file,
                r#"<!DOCTYPE svg PUBLIC "-//W3C//DTD SVG 1.1//EN" "http://www.w3.org/Graphics/SVG/1.1/DTD/svg11.dtd">"#
            )?;
        } else {
            write!(
                self.file,
                r#"<!DOCTYPE svg PUBLIC "-//W3C//DTD SVG 1.0//EN" "http://www.w3.org/TR/2001/REC-SVG-20010904/DTD/svg10.dtd">"#
            )?;
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
        writeln!(self.file)?;
        type Mat = Transform;

        let ctm = Mat::create_scale(1., -1.).post_translate(vec2(0., media_box.ury));
        writeln!(
            self.file,
            "<g transform='matrix({}, {}, {}, {}, {}, {})'>",
            ctm.m11, ctm.m12, ctm.m21, ctm.m22, ctm.m31, ctm.m32,
        )?;
        Ok(())
    }
    fn end_page(&mut self) -> Result<(), OutputError> {
        writeln!(self.file, "</g>")?;
        write!(self.file, "</svg>")?;
        Ok(())
    }
    fn output_character(
        &mut self,
        _trm: &Transform,
        _width: f64,
        _spacing: f64,
        _font_size: f64,
        _char: &str,
    ) -> Result<(), OutputError> {
        Ok(())
    }
    fn begin_word(&mut self) -> Result<(), OutputError> {
        Ok(())
    }
    fn end_word(&mut self) -> Result<(), OutputError> {
        Ok(())
    }
    fn end_line(&mut self) -> Result<(), OutputError> {
        Ok(())
    }
    fn fill(
        &mut self,
        ctm: &Transform,
        _colorspace: &ColorSpace,
        _color: &[f64],
        path: &Path,
    ) -> Result<(), OutputError> {
        write!(
            self.file,
            "<g transform='matrix({}, {}, {}, {}, {}, {})'>",
            ctm.m11, ctm.m12, ctm.m21, ctm.m22, ctm.m31, ctm.m32,
        )?;

        /*if path.ops.len() == 1 {
            if let PathOp::Rect(x, y, width, height) = path.ops[0] {
                write!(self.file, "<rect x={} y={} width={} height={} />\n", x, y, width, height);
                write!(self.file, "</g>");
                return;
            }
        }*/
        let mut d = Vec::new();
        for op in &path.ops {
            match *op {
                PathOp::MoveTo(x, y) => d.push(format!("M{x} {y}")),
                PathOp::LineTo(x, y) => d.push(format!("L{x} {y}")),
                PathOp::CurveTo(x1, y1, x2, y2, x, y) => {
                    d.push(format!("C{x1} {y1} {x2} {y2} {x} {y}"));
                }
                PathOp::Close => d.push("Z".to_string()),
                PathOp::Rect(x, y, width, height) => {
                    d.push(format!("M{x} {y}"));
                    d.push(format!("L{} {}", x + width, y));
                    d.push(format!("L{} {}", x + width, y + height));
                    d.push(format!("L{} {}", x, y + height));
                    d.push("Z".to_string());
                }
            }
        }
        write!(self.file, "<path d='{}' />", d.join(" "))?;
        write!(self.file, "</g>")?;
        writeln!(self.file)?;
        Ok(())
    }
}

/*
File doesn't implement std::fmt::Write so we have
to do some gymnastics to accept a File or String
See https://github.com/rust-lang/rust/issues/51305
*/

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
    fn write_str(&mut self, s: &str) -> Result<(), std::fmt::Error> {
        self.f.write_all(s.as_bytes()).map_err(|_| fmt::Error)
    }
}

impl ConvertToFmt for &mut dyn std::io::Write {
    type Writer = WriteAdapter<Self>;
    fn convert(self) -> Self::Writer {
        WriteAdapter { f: self }
    }
}

impl ConvertToFmt for &mut File {
    type Writer = WriteAdapter<Self>;
    fn convert(self) -> Self::Writer {
        WriteAdapter { f: self }
    }
}

pub struct PlainTextOutput<W: ConvertToFmt> {
    writer: W::Writer,
    last_end: f64,
    last_y: f64,
    first_char: bool,
    flip_ctm: Transform,
}

impl<W: ConvertToFmt> PlainTextOutput<W> {
    pub fn new(writer: W) -> Self {
        Self {
            writer: writer.convert(),
            last_end: 100_000.,
            first_char: false,
            last_y: 0.,
            flip_ctm: Transform2D::identity(),
        }
    }
}

/* There are some structural hints that PDFs can use to signal word and line endings:
 * however relying on these is not likely to be sufficient. */
impl<W: ConvertToFmt> OutputDev for PlainTextOutput<W> {
    fn begin_page(
        &mut self,
        _page_num: u32,
        media_box: &MediaBox,
        _: Option<ArtBox>,
    ) -> Result<(), OutputError> {
        self.flip_ctm = Transform2D::row_major(1., 0., 0., -1., 0., media_box.ury - media_box.lly);
        Ok(())
    }
    fn end_page(&mut self) -> Result<(), OutputError> {
        Ok(())
    }
    fn output_character(
        &mut self,
        trm: &Transform,
        width: f64,
        _spacing: f64,
        font_size: f64,
        char: &str,
    ) -> Result<(), OutputError> {
        let position = trm.post_transform(&self.flip_ctm);
        let transformed_font_size_vec = trm.transform_vector(vec2(font_size, font_size));
        // get the length of one sized of the square with the same area with a rectangle of size (x, y)
        let transformed_font_size =
            (transformed_font_size_vec.x * transformed_font_size_vec.y).sqrt();
        let (x, y) = (position.m31, position.m32);
        //dlog!("last_end: {} x: {}, width: {}", self.last_end, x, width);
        if self.first_char {
            if (y - self.last_y).abs() > transformed_font_size * 1.5 {
                writeln!(self.writer)?;
            }

            // we've moved to the left and down
            if x < self.last_end && (y - self.last_y).abs() > transformed_font_size * 0.5 {
                writeln!(self.writer)?;
            }

            if x > transformed_font_size.mul_add(0.1, self.last_end) {
                dlog!(
                    "width: {}, space: {}, thresh: {}",
                    width,
                    x - self.last_end,
                    transformed_font_size * 0.1
                );
                write!(self.writer, " ")?;
            }
        }
        //let norm = unicode_normalization::UnicodeNormalization::nfkc(char);
        write!(self.writer, "{char}")?;
        self.first_char = false;
        self.last_y = y;
        self.last_end = width.mul_add(transformed_font_size, x);
        Ok(())
    }
    fn begin_word(&mut self) -> Result<(), OutputError> {
        self.first_char = true;
        Ok(())
    }
    fn end_word(&mut self) -> Result<(), OutputError> {
        Ok(())
    }
    fn end_line(&mut self) -> Result<(), OutputError> {
        //write!(self.file, "\n");
        Ok(())
    }
}

pub fn print_metadata(doc: &Document) {
    dlog!("Version: {}", doc.version);
    if let Some(info) = get_info(doc) {
        for (k, v) in info {
            if let &Object::String(ref s, StringFormat::Literal) = v {
                dlog!("{}: {}", pdf_to_utf8(k), pdf_to_utf8(s));
            }
        }
    }
    dlog!("Page count: {}", get::<i64>(doc, get_pages(doc), b"Count"));
    dlog!("Pages: {:?}", get_pages(doc));
    dlog!(
        "Type: {:?}",
        get_pages(doc)
            .get(b"Type")
            .and_then(|x| x.as_name())
            .unwrap()
    );
}

/// Extract the text from a pdf at `path` and return a `String` with the results
pub fn extract_text<P: std::convert::AsRef<std::path::Path>>(
    path: P,
) -> Result<String, OutputError> {
    let mut s = String::new();
    {
        let mut output = PlainTextOutput::new(&mut s);
        let mut doc = Document::load(path)?;
        maybe_decrypt(&mut doc)?;
        output_doc(&doc, &mut output)?;
    }
    Ok(s)
}

fn maybe_decrypt(doc: &mut Document) -> Result<(), OutputError> {
    if !doc.is_encrypted() {
        return Ok(());
    }

    if let Err(e) = doc.decrypt("") {
        if matches!(e, Error::Decryption(DecryptionError::IncorrectPassword)) {
            eprintln!("Encrypted documents must be decrypted with a password using {{extract_text|extract_text_from_mem|output_doc}}_encrypted");
        }

        return Err(OutputError::PdfError(e));
    }

    Ok(())
}

pub fn extract_text_encrypted<P: std::convert::AsRef<std::path::Path>, PW: AsRef<[u8]>>(
    path: P,
    password: PW,
) -> Result<String, OutputError> {
    let mut s = String::new();
    {
        let mut output = PlainTextOutput::new(&mut s);
        let mut doc = Document::load(path)?;
        output_doc_encrypted(&mut doc, &mut output, password)?;
    }
    Ok(s)
}

pub fn extract_text_from_mem(buffer: &[u8]) -> Result<String, OutputError> {
    let mut s = String::new();
    {
        let mut output = PlainTextOutput::new(&mut s);
        let mut doc = Document::load_mem(buffer)?;
        maybe_decrypt(&mut doc)?;
        output_doc(&doc, &mut output)?;
    }
    Ok(s)
}

pub fn extract_text_from_mem_encrypted<PW: AsRef<[u8]>>(
    buffer: &[u8],
    password: PW,
) -> Result<String, OutputError> {
    let mut s = String::new();
    {
        let mut output = PlainTextOutput::new(&mut s);
        let mut doc = Document::load_mem(buffer)?;
        output_doc_encrypted(&mut doc, &mut output, password)?;
    }
    Ok(s)
}

fn extract_text_by_page(doc: &Document, page_num: u32) -> Result<String, OutputError> {
    let mut s = String::new();
    {
        let mut output = PlainTextOutput::new(&mut s);
        output_doc_page(doc, &mut output, page_num)?;
    }
    Ok(s)
}

/// Extract the text from a pdf at `path` and return a `Vec<String>` with the results separately by page
pub fn extract_text_by_pages<P: std::convert::AsRef<std::path::Path>>(
    path: P,
) -> Result<Vec<String>, OutputError> {
    let mut v = Vec::new();
    {
        let mut doc = Document::load(path)?;
        maybe_decrypt(&mut doc)?;
        let mut page_num = 1;
        while let Ok(content) = extract_text_by_page(&doc, page_num) {
            v.push(content);
            page_num += 1;
        }
    }
    Ok(v)
}

pub fn extract_text_by_pages_encrypted<P: std::convert::AsRef<std::path::Path>, PW: AsRef<[u8]>>(
    path: P,
    password: PW,
) -> Result<Vec<String>, OutputError> {
    let mut v = Vec::new();
    {
        let mut doc = Document::load(path)?;
        doc.decrypt(password)?;
        let mut page_num = 1;
        while let Ok(content) = extract_text_by_page(&doc, page_num) {
            v.push(content);
            page_num += 1;
        }
    }
    Ok(v)
}

pub fn extract_text_from_mem_by_pages(buffer: &[u8]) -> Result<Vec<String>, OutputError> {
    let mut v = Vec::new();
    {
        let mut doc = Document::load_mem(buffer)?;
        maybe_decrypt(&mut doc)?;
        let mut page_num = 1;
        while let Ok(content) = extract_text_by_page(&doc, page_num) {
            v.push(content);
            page_num += 1;
        }
    }
    Ok(v)
}

pub fn extract_text_from_mem_by_pages_encrypted<PW: AsRef<[u8]>>(
    buffer: &[u8],
    password: PW,
) -> Result<Vec<String>, OutputError> {
    let mut v = Vec::new();
    {
        let mut doc = Document::load_mem(buffer)?;
        doc.decrypt(password)?;
        let mut page_num = 1;
        while let Ok(content) = extract_text_by_page(&doc, page_num) {
            v.push(content);
            page_num += 1;
        }
    }
    Ok(v)
}

fn get_inherited<'a, T: FromObj<'a>>(
    doc: &'a Document,
    dict: &'a Dictionary,
    key: &[u8],
) -> Option<T> {
    let o: Option<T> = get(doc, dict, key);
    if let Some(o) = o {
        Some(o)
    } else {
        let parent = dict
            .get(b"Parent")
            .and_then(lopdf::Object::as_reference)
            .and_then(|id| doc.get_dictionary(id))
            .ok()?;
        get_inherited(doc, parent, key)
    }
}

pub fn output_doc_encrypted<PW: AsRef<[u8]>>(
    doc: &mut Document,
    output: &mut dyn OutputDev,
    password: PW,
) -> Result<(), OutputError> {
    doc.decrypt(password)?;
    output_doc(doc, output)
}

/// Parse a given document and output it to `output`
pub fn output_doc(doc: &Document, output: &mut dyn OutputDev) -> Result<(), OutputError> {
    if doc.is_encrypted() {
        eprintln!("Encrypted documents must be decrypted with a password using {{extract_text|extract_text_from_mem|output_doc}}_encrypted");
    }
    let empty_resources = Dictionary::new();
    let pages = doc.get_pages();
    let mut p = Processor::new();
    for dict in pages {
        let page_num = dict.0;
        let object_id = dict.1;
        output_doc_inner(page_num, object_id, doc, &mut p, output, &empty_resources)?;
    }
    Ok(())
}

pub fn output_doc_page(
    doc: &Document,
    output: &mut dyn OutputDev,
    page_num: u32,
) -> Result<(), OutputError> {
    if doc.is_encrypted() {
        eprintln!("Encrypted documents must be decrypted with a password using {{extract_text|extract_text_from_mem|output_doc}}_encrypted");
    }
    let empty_resources = Dictionary::new();
    let pages = doc.get_pages();
    let object_id = pages
        .get(&page_num)
        .ok_or(lopdf::Error::PageNumberNotFound(page_num))?;
    let mut p = Processor::new();
    output_doc_inner(page_num, *object_id, doc, &mut p, output, &empty_resources)?;
    Ok(())
}

fn output_doc_inner<'a>(
    page_num: u32,
    object_id: ObjectId,
    doc: &'a Document,
    p: &mut Processor<'a>,
    output: &mut dyn OutputDev,
    empty_resources: &'a Dictionary,
) -> Result<(), OutputError> {
    let page_dict = doc.get_object(object_id).unwrap().as_dict().unwrap();
    dlog!("page {} {:?}", page_num, page_dict);
    // XXX: Some pdfs lack a Resources directory
    let resources = get_inherited(doc, page_dict, b"Resources").unwrap_or(empty_resources);
    dlog!("resources {:?}", resources);
    // pdfium searches up the page tree for MediaBoxes as needed
    let media_box: Vec<f64> = get_inherited(doc, page_dict, b"MediaBox").expect("MediaBox");
    let media_box = MediaBox {
        llx: media_box[0],
        lly: media_box[1],
        urx: media_box[2],
        ury: media_box[3],
    };
    let art_box =
        get::<Option<Vec<f64>>>(doc, page_dict, b"ArtBox").map(|x| (x[0], x[1], x[2], x[3]));
    output.begin_page(page_num, &media_box, art_box)?;
    p.process_stream(
        doc,
        &doc.get_page_content(object_id).unwrap(),
        resources,
        &media_box,
        output,
        page_num,
    )?;
    output.end_page()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::iter::FromIterator;
    use tempfile::NamedTempFile;

    use lopdf::content::Operation;

    // Helper function to create a simple PDF document for testing
    fn create_test_pdf() -> Vec<u8> {
        let mut doc = Document::with_version("1.5");

        // Add a simple page with text
        let pages = Dictionary::from_iter(vec![
            (b"Type".to_vec(), Object::Name(b"Pages".to_vec())),
            (b"Count".to_vec(), Object::Integer(1)),
            (
                b"MediaBox".to_vec(),
                Object::Array(vec![
                    Object::Integer(0),
                    Object::Integer(0),
                    Object::Integer(612),
                    Object::Integer(792),
                ]),
            ),
        ]);
        let pages_id = doc.add_object(pages);

        let font_dict = Dictionary::from_iter(vec![
            (b"Type".to_vec(), Object::Name(b"Font".to_vec())),
            (b"Subtype".to_vec(), Object::Name(b"Type1".to_vec())),
            (b"BaseFont".to_vec(), Object::Name(b"Helvetica".to_vec())),
        ]);
        let font_id = doc.add_object(font_dict);

        let resources = Dictionary::from_iter(vec![(
            b"Font".to_vec(),
            Object::Dictionary(Dictionary::from_iter(vec![(
                b"F1".to_vec(),
                Object::Reference(font_id),
            )])),
        )]);

        let content = Content {
            operations: vec![
                Operation::new("BT", vec![]),
                Operation::new(
                    "Tf",
                    vec![Object::Name(b"F1".to_vec()), Object::Integer(12)],
                ),
                Operation::new("Td", vec![Object::Integer(100), Object::Integer(700)]),
                Operation::new(
                    "Tj",
                    vec![Object::String(
                        b"Test Content".to_vec(),
                        StringFormat::Literal,
                    )],
                ),
                Operation::new("ET", vec![]),
            ],
        };

        let content_stream = Stream::new(Dictionary::new(), content.encode().unwrap());
        let content_id = doc.add_object(content_stream);

        let page = Dictionary::from_iter(vec![
            (b"Type".to_vec(), Object::Name(b"Page".to_vec())),
            (b"Parent".to_vec(), Object::Reference(pages_id)),
            (b"Resources".to_vec(), Object::Dictionary(resources)),
            (b"Contents".to_vec(), Object::Reference(content_id)),
        ]);
        let page_id = doc.add_object(page);

        // Update pages with kids
        doc.get_object_mut(pages_id)
            .unwrap()
            .as_dict_mut()
            .unwrap()
            .set("Kids", Object::Array(vec![Object::Reference(page_id)]));

        // Set up document catalog
        let catalog = Dictionary::from_iter(vec![
            (b"Type".to_vec(), Object::Name(b"Catalog".to_vec())),
            (b"Pages".to_vec(), Object::Reference(pages_id)),
        ]);
        let catalog_id = doc.add_object(catalog);
        doc.trailer.set("Root", Object::Reference(catalog_id));

        // Save to memory
        let mut buffer = Vec::new();
        doc.save_to(&mut buffer).unwrap();
        buffer
    }

    #[test]
    fn test_basic_text_extraction() {
        let pdf_data = create_test_pdf();
        let result = extract_text_from_mem(&pdf_data).unwrap();
        assert!(result.contains("Test Content"));
    }

    #[test]
    fn test_extract_text_by_pages() {
        let pdf_data = create_test_pdf();
        let pages = extract_text_from_mem_by_pages(&pdf_data).unwrap();
        assert_eq!(pages.len(), 1);
        assert!(pages[0].contains("Test Content"));
    }

    #[test]
    fn test_plain_text_output() {
        let mut output = String::new();
        let mut text_output = PlainTextOutput::new(&mut output);

        // Test basic character output
        text_output
            .begin_page(
                1,
                &MediaBox {
                    llx: 0.0,
                    lly: 0.0,
                    urx: 612.0,
                    ury: 792.0,
                },
                None,
            )
            .unwrap();
        text_output.begin_word().unwrap();
        text_output
            .output_character(&Transform2D::identity(), 10.0, 0.0, 12.0, "Test")
            .unwrap();
        text_output.end_word().unwrap();
        text_output.end_page().unwrap();

        assert!(output.contains("Test"));
    }

    #[test]
    fn test_pdf_to_utf8_conversion() {
        // Test basic ASCII
        let input = b"Hello";
        assert_eq!(pdf_to_utf8(input), "Hello");

        // Test UTF-16BE with BOM
        let mut input = vec![0xFE, 0xFF];
        input.extend_from_slice(&[0x00, 0x41]); // Letter 'A' in UTF-16BE
        assert_eq!(pdf_to_utf8(&input), "A");
    }

    #[test]
    fn test_get_unicode_map() {
        let doc = Document::with_version("1.5");
        let font_dict = Dictionary::new();
        let result = get_unicode_map(&doc, &font_dict);
        assert!(result.is_none()); // Should return None for empty dictionary
    }

    #[test]
    fn test_file_handling() {
        // Test with temporary file
        let mut temp_file = NamedTempFile::new().unwrap();
        let pdf_data = create_test_pdf();
        temp_file.write_all(&pdf_data).unwrap();

        let result = extract_text(temp_file.path()).unwrap();
        assert!(result.contains("Test Content"));
    }

    #[test]
    fn test_invalid_pdf() {
        let invalid_data = b"This is not a PDF file";
        let result = extract_text_from_mem(invalid_data);
        assert!(result.is_err());
    }
}
