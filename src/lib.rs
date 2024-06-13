#![allow(dead_code)]
use adobe_cmap_parser::{ByteMapping, CIDRange, CodeRange};
use anyhow::Result;
use encoding::all::UTF_16BE;
use encoding::{DecoderTrap, Encoding};
use euclid::vec2;
use euclid::*;
use lopdf::content::Content;
use lopdf::encryption::DecryptionError;
use lopdf::*;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::fmt;
use std::fmt::Debug;
use std::fmt::Write;
use std::fs::File;
use std::rc::Rc;
use std::slice::Iter;
use std::str;
use thiserror::Error;
use tracing::{debug, error, trace};
use unicode_normalization::UnicodeNormalization;

mod core_fonts;
mod encodings;
mod glyphnames;
mod zapfglyphnames;

pub struct Space;
pub type Transform = Transform2D<f64, Space, Space>;

#[derive(Error, Debug)]
pub enum PdfExtractError {
    #[error(transparent)]
    FormatError(#[from] std::fmt::Error),
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    PdfError(#[from] lopdf::Error),

    #[error("file is encrypted")]
    Encrypted,

    #[error("{0}")]
    Error(String),
}

fn get_info(doc: &Document) -> Option<&Dictionary> {
    if let Ok(Object::Reference(ref id)) = doc.trailer.get(b"Info") {
        if let Ok(Object::Dictionary(ref info)) = doc.get_object(*id) {
            return Some(info);
        }
    }

    None
}

fn get_catalog(doc: &Document) -> Result<&Dictionary> {
    if let Object::Reference(ref id) = doc.trailer.get(b"Root")? {
        if let Ok(Object::Dictionary(ref catalog)) = doc.get_object(*id) {
            return Ok(catalog);
        }
    }

    Err(PdfExtractError::Error("could not get catalog".into()))?
}

fn get_pages(doc: &Document) -> Result<&Dictionary> {
    let catalog = get_catalog(doc)?;

    match catalog.get(b"Pages")? {
        Object::Reference(ref id) => match doc.get_object(*id) {
            Ok(Object::Dictionary(ref pages)) => {
                return Ok(pages);
            }
            other => {
                trace!("pages: {:?}", other)
            }
        },
        other => {
            trace!("pages: {:?}", other)
        }
    }

    trace!("catalog {:?}", catalog);

    Err(PdfExtractError::Error(format!(
        "could not get catalog {catalog:?}"
    )))?
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

fn pdf_to_utf8(s: &[u8]) -> Result<String> {
    let result = if s.len() > 2 && s[0] == 0xfe && s[1] == 0xff {
        UTF_16BE
            .decode(&s[2..], DecoderTrap::Strict)
            .map_err(|error| PdfExtractError::Error(format!("{error:?}")))
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
            .decode(&r, DecoderTrap::Strict)
            .map_err(|error| PdfExtractError::Error(format!("{error:?}")))
    };

    Ok(result?)
}

fn to_utf8(encoding: &[u16], s: &[u8]) -> Result<String> {
    let result = if s.len() > 2 && s[0] == 0xfe && s[1] == 0xff {
        UTF_16BE
            .decode(&s[2..], DecoderTrap::Strict)
            .map_err(|error| PdfExtractError::Error(format!("{error:?}")))
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
            .decode(&r, DecoderTrap::Strict)
            .map_err(|error| PdfExtractError::Error(format!("{error:?}")))
    };

    Ok(result?)
}

fn maybe_deref<'a>(doc: &'a Document, o: &'a Object) -> Result<&'a Object> {
    let result = match o {
        Object::Reference(r) => doc
            .get_object(*r)
            .map_err(|_error| PdfExtractError::Error("missing object reference".into()))?,
        _ => o,
    };

    Ok(result)
}

fn maybe_get_obj<'a>(doc: &'a Document, dict: &'a Dictionary, key: &[u8]) -> Option<&'a Object> {
    dict.get(key)
        .map(|o| maybe_deref(doc, o).ok())
        .ok()
        .flatten()
}

// an intermediate trait that can be used to chain conversions that may have failed
trait FromOptObj<'a> {
    fn from_opt_obj(doc: &'a Document, obj: Option<&'a Object>, key: &[u8]) -> Result<Self>
    where
        Self: Sized;
}

// conditionally convert to Self returns None if the conversion failed
trait FromObj<'a>
where
    Self: std::marker::Sized,
{
    fn from_obj(doc: &'a Document, obj: &'a Object) -> Result<Option<Self>>;
}

impl<'a, T: FromObj<'a>> FromOptObj<'a> for Option<T> {
    fn from_opt_obj(doc: &'a Document, obj: Option<&'a Object>, _key: &[u8]) -> Result<Self> {
        let Some(object) = obj else {
            return Ok(None);
        };

        T::from_obj(doc, object)
    }
}

impl<'a, T: FromObj<'a>> FromOptObj<'a> for T {
    fn from_opt_obj(doc: &'a Document, obj: Option<&'a Object>, key: &[u8]) -> Result<Self> {
        T::from_obj(
            doc,
            obj.ok_or(PdfExtractError::Error(format!(
                "{}",
                String::from_utf8_lossy(key)
            )))?,
        )?
        .ok_or(PdfExtractError::Error("wrong type".into()).into())
    }
}

// we follow the same conventions as pdfium for when to support indirect objects:
// on arrays, streams and dicts
impl<'a, T: FromObj<'a>> FromObj<'a> for Vec<T> {
    fn from_obj(doc: &'a Document, obj: &'a Object) -> Result<Option<Self>> {
        let Ok(entries) = maybe_deref(doc, obj)?.as_array() else {
            return Ok(None);
        };

        let mut array = entries
            .iter()
            .map(|entry| T::from_obj(doc, entry))
            .collect::<Result<Option<Vec<_>>>>()?;

        Ok(array.take())
    }
}

// XXX: These will panic if we don't have the right number of items
// we don't want to do that
impl<'a, T: FromObj<'a>> FromObj<'a> for [T; 4] {
    fn from_obj(doc: &'a Document, obj: &'a Object) -> Result<Option<Self>> {
        let entries = match maybe_deref(doc, obj)?.as_array() {
            Ok(entries) if entries.len() == 4 => entries,
            _ => return Ok(None),
        };

        let mut array = entries
            .iter()
            .map(|entry| T::from_obj(doc, entry))
            .collect::<Result<Option<Vec<_>>>>()?;

        if let Some(array) = array.take() {
            if let Ok(array) = array.try_into() {
                return Ok(Some(array));
            }
        }

        Ok(None)
    }
}

impl<'a, T: FromObj<'a>> FromObj<'a> for [T; 3] {
    fn from_obj(doc: &'a Document, obj: &'a Object) -> Result<Option<Self>> {
        let entries = match maybe_deref(doc, obj)?.as_array() {
            Ok(entries) if entries.len() == 3 => entries,
            _ => return Ok(None),
        };

        let mut array = entries
            .iter()
            .map(|entry| T::from_obj(doc, entry))
            .collect::<Result<Option<Vec<_>>>>()?;

        if let Some(array) = array.take() {
            if let Ok(array) = array.try_into() {
                return Ok(Some(array));
            }
        }

        Ok(None)
    }
}

impl<'a> FromObj<'a> for f64 {
    fn from_obj(_doc: &Document, obj: &Object) -> Result<Option<Self>> {
        Ok(match obj {
            Object::Integer(i) => Some(*i as f64),
            Object::Real(f) => Some((*f).into()),
            _ => None,
        })
    }
}

impl<'a> FromObj<'a> for i64 {
    fn from_obj(_doc: &Document, obj: &Object) -> Result<Option<Self>> {
        Ok(match obj {
            Object::Integer(i) => Some(*i),
            _ => None,
        })
    }
}

impl<'a> FromObj<'a> for &'a Dictionary {
    fn from_obj(doc: &'a Document, obj: &'a Object) -> Result<Option<&'a Dictionary>> {
        Ok(maybe_deref(doc, obj)?.as_dict().ok())
    }
}

impl<'a> FromObj<'a> for &'a Stream {
    fn from_obj(doc: &'a Document, obj: &'a Object) -> Result<Option<&'a Stream>> {
        Ok(maybe_deref(doc, obj)?.as_stream().ok())
    }
}

impl<'a> FromObj<'a> for &'a Object {
    fn from_obj(doc: &'a Document, obj: &'a Object) -> Result<Option<&'a Object>> {
        Ok(Some(maybe_deref(doc, obj)?))
    }
}

fn get<'a, T: FromOptObj<'a>>(doc: &'a Document, dict: &'a Dictionary, key: &[u8]) -> Result<T> {
    T::from_opt_obj(doc, dict.get(key).ok(), key)
}

fn maybe_get<'a, T: FromObj<'a>>(
    doc: &'a Document,
    dict: &'a Dictionary,
    key: &[u8],
) -> Result<Option<T>> {
    let Some(object) = maybe_get_obj(doc, dict, key) else {
        return Ok(None);
    };

    T::from_obj(doc, object)
}

fn get_name_string<'a>(doc: &'a Document, dict: &'a Dictionary, key: &[u8]) -> Result<String> {
    pdf_to_utf8(
        dict.get(key)
            .map(|o| maybe_deref(doc, o))?
            .map_err(|_| PdfExtractError::Error("deref failed".into()))?
            .as_name()
            .map_err(|_| PdfExtractError::Error("get name failed".into()))?,
    )
}

fn maybe_get_name_string<'a>(
    doc: &'a Document,
    dict: &'a Dictionary,
    key: &[u8],
) -> Result<Option<String>> {
    maybe_get_obj(doc, dict, key)
        .and_then(|n| n.as_name().ok())
        .map(pdf_to_utf8)
        .transpose()
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

fn make_font<'a>(doc: &'a Document, font: &'a Dictionary) -> Result<Rc<dyn PdfFont + 'a>> {
    let subtype = get_name_string(doc, font, b"Subtype")?;

    trace!("MakeFont({})", subtype);

    Ok(if subtype == "Type0" {
        Rc::new(PdfCIDFont::new(doc, font)?)
    } else if subtype == "Type3" {
        Rc::new(PdfType3Font::new(doc, font)?)
    } else {
        Rc::new(PdfSimpleFont::new(doc, font)?)
    })
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

fn encoding_to_unicode_table(name: &[u8]) -> Result<Vec<u16>> {
    let encoding = match name {
        b"MacRomanEncoding" => encodings::MAC_ROMAN_ENCODING,
        b"MacExpertEncoding" => encodings::MAC_EXPERT_ENCODING,
        b"WinAnsiEncoding" => encodings::WIN_ANSI_ENCODING,
        _ => Err(PdfExtractError::Error(format!(
            "unexpected encoding {:?}",
            pdf_to_utf8(name)?
        )))?,
    };

    let encoding_table = encoding
        .iter()
        .map(|x| {
            if let Some(x) = x {
                Ok(glyphnames::name_to_unicode(x).ok_or(PdfExtractError::Error(
                    "could not convert name to unicode".into(),
                ))?)
            } else {
                Ok(0)
            }
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(encoding_table)
}

/* "Glyphs in the font are selected by single-byte character codes obtained from a string that
    is shown by the text-showing operators. Logically, these codes index into a table of 256
    glyphs; the mapping from codes to glyphs is called the font’s encoding. Each font program
    has a built-in encoding. Under some circumstances, the encoding can be altered by means
    described in Section 5.5.5, “Character Encoding.”
*/
impl<'a> PdfSimpleFont<'a> {
    fn new(doc: &'a Document, font: &'a Dictionary) -> Result<PdfSimpleFont<'a>> {
        let base_name = get_name_string(doc, font, b"BaseFont")?;
        let subtype = get_name_string(doc, font, b"Subtype")?;

        let encoding: Option<&Object> = get(doc, font, b"Encoding")?;

        trace!(
            "base_name {} {} enc:{:?} {:?}",
            base_name,
            subtype,
            encoding,
            font
        );

        let descriptor: Option<&Dictionary> = get(doc, font, b"FontDescriptor")?;
        let mut type1_encoding = None;

        if let Some(descriptor) = descriptor {
            trace!("descriptor {:?}", descriptor);

            if subtype == "Type1" {
                let file = maybe_get_obj(doc, descriptor, b"FontFile");

                match file {
                    Some(Object::Stream(ref s)) => {
                        let s = get_contents(s);

                        //trace!("font contents {:?}", pdf_to_utf8(&s));

                        type1_encoding = Some(
                            type1_encoding_parser::get_encoding_map(&s).map_err(|error| {
                                PdfExtractError::Error(format!("type1 encoding error: {error}"))
                            })?,
                        );
                    }
                    _ => {
                        trace!("font file {:?}", file)
                    }
                }
            } else if subtype == "TrueType" {
                let file = maybe_get_obj(doc, descriptor, b"FontFile2");

                match file {
                    Some(Object::Stream(ref s)) => {
                        let _s = get_contents(s);

                        //File::create(format!("/tmp/{}", base_name)).unwrap().write_all(&s);
                    }
                    _ => {
                        trace!("font file {:?}", file)
                    }
                }
            }

            let font_file3 = get::<Option<&Object>>(doc, descriptor, b"FontFile3")?;

            match font_file3 {
                Some(Object::Stream(ref s)) => {
                    let subtype = get_name_string(doc, &s.dict, b"Subtype")?;

                    trace!("font file {}, {:?}", subtype, s);
                }
                None => {}
                _ => {
                    trace!("unexpected")
                }
            }

            let charset = maybe_get_obj(doc, descriptor, b"CharSet");

            let _charset = match charset {
                Some(Object::String(ref s, _)) => Some(pdf_to_utf8(s)?),
                _ => None,
            };
            //trace!("charset {:?}", charset);
        }

        let mut unicode_map = get_unicode_map(doc, font)?;
        let mut encoding_table = None;

        match encoding {
            Some(Object::Name(ref encoding_name)) => {
                trace!("encoding {:?}", pdf_to_utf8(encoding_name)?);

                encoding_table = Some(encoding_to_unicode_table(encoding_name)?);
            }
            Some(Object::Dictionary(ref encoding)) => {
                //trace!("Encoding {:?}", encoding);

                let mut table =
                    if let Some(base_encoding) = maybe_get_name(doc, encoding, b"BaseEncoding") {
                        trace!("BaseEncoding {:?}", base_encoding);

                        encoding_to_unicode_table(base_encoding)?
                    } else {
                        Vec::from(PDFDocEncoding)
                    };

                let differences = maybe_get_array(doc, encoding, b"Differences");

                if let Some(differences) = differences {
                    trace!("Differences");

                    let mut code = 0;

                    for o in differences {
                        let o = maybe_deref(doc, o)?;

                        match o {
                            Object::Integer(i) => {
                                code = *i;
                            }
                            Object::Name(ref n) => {
                                let name = pdf_to_utf8(n)?;

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
                                                v.insert(String::from_utf16(&be)?);
                                            }
                                            Entry::Occupied(e) => {
                                                if e.get() != &String::from_utf16(&be)? {
                                                    let normal_match = e
                                                        .get()
                                                        .nfkc()
                                                        .eq(String::from_utf16(&be)?.nfkc());

                                                    trace!(
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
                                                    v.insert("".to_owned());
                                                }
                                                Entry::Occupied(e) => Err(PdfExtractError::Error(
                                                    format!("unicode failed: {e:?}"),
                                                ))?,
                                            }
                                        }
                                        _ => {
                                            trace!(
                                                "unknown glyph name '{}' for font {}",
                                                name,
                                                base_name
                                            );
                                        }
                                    }
                                }

                                trace!("{} = {} ({:?})", code, name, unicode);

                                if let Some(ref mut unicode_map) = unicode_map {
                                    // The unicode map might not have the code in it, but the code might
                                    // not be used so we don't want to panic here.
                                    // An example of this is the 'suppress' character in the TeX Latin Modern font.
                                    // This shows up in https://arxiv.org/pdf/2405.01295v1.pdf
                                    trace!("{} {:?}", code, unicode_map.get(&(code as u32)));
                                }

                                code += 1;
                            }
                            _ => Err(PdfExtractError::Error(format!("wrong type {:?}", o)))?,
                        }
                    }
                }

                let name = pdf_to_utf8(encoding.get(b"Type")?.as_name()?)?;

                trace!("name: {}", name);

                encoding_table = Some(table);
            }
            None => {
                if let Some(type1_encoding) = type1_encoding {
                    let mut table = Vec::from(PDFDocEncoding);

                    trace!("type1encoding");

                    for (code, name) in type1_encoding {
                        let unicode = glyphnames::name_to_unicode(&pdf_to_utf8(&name)?);

                        if let Some(unicode) = unicode {
                            table[code as usize] = unicode;
                        } else {
                            trace!("unknown character {}", pdf_to_utf8(&name)?);
                        }
                    }

                    encoding_table = Some(table)
                } else if subtype == "TrueType" {
                    encoding_table = Some(
                        encodings::WIN_ANSI_ENCODING
                            .iter()
                            .map(|x| {
                                if let &Some(x) = x {
                                    glyphnames::name_to_unicode(x)
                                        .ok_or(PdfExtractError::Error("no value".into()).into())
                                } else {
                                    Ok(0)
                                }
                            })
                            .collect::<Result<Vec<u16>>>()?,
                    );
                }
            }
            _ => Err(PdfExtractError::Error("unknown encoding".into()))?,
        }

        let mut width_map = HashMap::new();

        /* "Ordinarily, a font dictionary that refers to one of the standard fonts
        should omit the FirstChar, LastChar, Widths, and FontDescriptor entries.
        However, it is permissible to override a standard font by including these
        entries and embedding the font program in the PDF file."

        Note: some PDFs include a descriptor but still don't include these entries */

        // If we have widths prefer them over the core font widths. Needed for https://dkp.de/wp-content/uploads/parteitage/Sozialismusvorstellungen-der-DKP.pdf
        if let (Some(first_char), Some(last_char), Some(widths)) = (
            maybe_get::<i64>(doc, font, b"FirstChar")?,
            maybe_get::<i64>(doc, font, b"LastChar")?,
            maybe_get::<Vec<f64>>(doc, font, b"Widths")?,
        ) {
            // Some PDF's don't have these like fips-197.pdf
            let mut i: i64 = 0;

            trace!(
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

            if first_char + i - 1 != last_char {
                Err(PdfExtractError::Error("invalid widths".into()))?
            }
        } else if is_core_font(&base_name) {
            for font_metrics in core_fonts::metrics().iter() {
                if font_metrics.0 == base_name {
                    if let Some(ref encoding) = encoding_table {
                        trace!("has encoding");

                        for w in font_metrics.2 {
                            let c = glyphnames::name_to_unicode(w.2)
                                .ok_or(PdfExtractError::Error("name to unicode failed".into()))?;

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
                            trace!("{} {}", w.0, w.2);

                            // -1 is "not encoded"
                            if w.0 != -1 {
                                table[w.0 as usize] = if base_name == "ZapfDingbats" {
                                    zapfglyphnames::zapfdigbats_names_to_unicode(w.2).ok_or(
                                        PdfExtractError::Error(
                                            "zapfdigbats names to unicode failed".into(),
                                        ),
                                    )?
                                } else {
                                    glyphnames::name_to_unicode(w.2).ok_or(
                                        PdfExtractError::Error("name to unicode failed".into()),
                                    )?
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
        } else {
            Err(PdfExtractError::Error("no widths".into()))?
        }

        let missing_width = get::<Option<f64>>(doc, font, b"MissingWidth")?.unwrap_or(0.);

        Ok(PdfSimpleFont {
            doc,
            font,
            widths: width_map,
            encoding: encoding_table,
            missing_width,
            unicode_map,
        })
    }

    fn get_type(&self) -> Result<String> {
        get_name_string(self.doc, self.font, b"Type")
    }

    fn get_basefont(&self) -> Result<String> {
        get_name_string(self.doc, self.font, b"BaseFont")
    }

    fn get_subtype(&self) -> Result<String> {
        get_name_string(self.doc, self.font, b"Subtype")
    }

    fn get_widths(&self) -> Result<Option<&Vec<Object>>> {
        Ok(maybe_get_obj(self.doc, self.font, b"Widths")
            .map(|widths| widths.as_array())
            .transpose()
            .map_err(|error| {
                PdfExtractError::Error(format!("Widths should be an array: {error:?}"))
            })?)
    }

    /* For type1: This entry is obsolescent and its use is no longer recommended. (See
     * implementation note 42 in Appendix H.) */
    fn get_name(&self) -> Result<Option<String>> {
        maybe_get_name_string(self.doc, self.font, b"Name")
    }

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
    fn new(doc: &'a Document, font: &'a Dictionary) -> Result<PdfType3Font<'a>> {
        let unicode_map = get_unicode_map(doc, font)?;
        let encoding: Option<&Object> = get(doc, font, b"Encoding")?;

        let encoding_table = match encoding {
            Some(Object::Name(ref encoding_name)) => {
                trace!("encoding {:?}", pdf_to_utf8(encoding_name)?);

                Some(encoding_to_unicode_table(encoding_name)?)
            }
            Some(Object::Dictionary(ref encoding)) => {
                //trace!("Encoding {:?}", encoding);
                let mut table =
                    if let Some(base_encoding) = maybe_get_name(doc, encoding, b"BaseEncoding") {
                        trace!("BaseEncoding {:?}", base_encoding);

                        encoding_to_unicode_table(base_encoding)?
                    } else {
                        Vec::from(PDFDocEncoding)
                    };

                let differences = maybe_get_array(doc, encoding, b"Differences");

                if let Some(differences) = differences {
                    trace!("Differences");

                    let mut code = 0;

                    for o in differences {
                        match o {
                            Object::Integer(i) => {
                                code = *i;
                            }
                            Object::Name(ref n) => {
                                let name = pdf_to_utf8(n)?;

                                // XXX: names of Type1 fonts can map to arbitrary strings instead of real
                                // unicode names, so we should probably handle this differently
                                let unicode = glyphnames::name_to_unicode(&name);

                                if let Some(unicode) = unicode {
                                    table[code as usize] = unicode;
                                }

                                trace!("{} = {} ({:?})", code, name, unicode);

                                if let Some(ref unicode_map) = unicode_map {
                                    trace!("{} {:?}", code, unicode_map.get(&(code as u32)));
                                }

                                code += 1;
                            }
                            _ => Err(PdfExtractError::Error("wrong type".into()))?,
                        }
                    }
                }

                let name_encoded = encoding.get(b"Type");

                if let Ok(Object::Name(name)) = name_encoded {
                    trace!("name: {}", pdf_to_utf8(name)?);
                } else {
                    trace!("name not found");
                }

                Some(table)
            }
            _ => Err(PdfExtractError::Error("wrong encoding".into()))?,
        };

        let first_char: i64 = get(doc, font, b"FirstChar")?;
        let last_char: i64 = get(doc, font, b"LastChar")?;
        let widths: Vec<f64> = get(doc, font, b"Widths")?;

        let mut width_map = HashMap::new();
        let mut i = 0;

        trace!(
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

        if first_char + i - 1 != last_char {
            Err(PdfExtractError::Error("wrong widths".into()))?
        }

        Ok(PdfType3Font {
            doc,
            font,
            widths: width_map,
            encoding: encoding_table,
            unicode_map,
        })
    }
}

type CharCode = u32;

struct PdfFontIter<'a> {
    i: Iter<'a, u8>,
    font: &'a dyn PdfFont,
}

impl<'a> Iterator for PdfFontIter<'a> {
    type Item = (CharCode, u8);

    fn next(&mut self) -> Option<(CharCode, u8)> {
        self.font.next_char(&mut self.i)
    }
}

trait PdfFont: Debug {
    fn get_width(&self, id: CharCode) -> Result<f64>;

    fn next_char(&self, iter: &mut Iter<u8>) -> Option<(CharCode, u8)>;

    fn decode_char(&self, char: CharCode) -> Result<String>;

    /*fn char_codes<'a>(&'a self, chars: &'a [u8]) -> PdfFontIter {
        let p = self;

        PdfFontIter{i: chars.iter(), font: p as &PdfFont}
    }*/
}

impl<'a> dyn PdfFont + 'a {
    fn char_codes(&'a self, chars: &'a [u8]) -> PdfFontIter {
        PdfFontIter {
            i: chars.iter(),
            font: self,
        }
    }

    fn decode(&self, chars: &[u8]) -> Result<String> {
        let strings = self
            .char_codes(chars)
            .map(|x| self.decode_char(x.0))
            .collect::<Result<Vec<_>>>()?;

        Ok(strings.join(""))
    }
}

impl<'a> PdfFont for PdfSimpleFont<'a> {
    fn get_width(&self, id: CharCode) -> Result<f64> {
        let width = self.widths.get(&id);

        if let Some(width) = width {
            Ok(*width)
        } else {
            let mut widths = self.widths.iter().collect::<Vec<_>>();

            widths.sort_by_key(|x| x.0);

            trace!(
                "missing width for {} len(widths) = {}, {:?} falling back to missing_width {:?}",
                id,
                self.widths.len(),
                widths,
                self.font
            );

            Ok(self.missing_width)
        }
    }

    /*fn decode(&self, chars: &[u8]) -> String {
        let encoding = self.encoding.as_ref().map(|x| &x[..]).unwrap_or(&PDFDocEncoding);
        to_utf8(encoding, chars)
    }*/

    fn next_char(&self, iter: &mut Iter<u8>) -> Option<(CharCode, u8)> {
        iter.next().map(|x| (*x as CharCode, 1))
    }

    fn decode_char(&self, char: CharCode) -> Result<String> {
        let slice = [char as u8];

        if let Some(ref unicode_map) = self.unicode_map {
            let s = unicode_map.get(&char);

            let s =
                match s {
                    None => {
                        trace!(
                            "missing char {:?} in unicode map {:?} for {:?}",
                            char,
                            unicode_map,
                            self.font
                        );

                        // some pdf's like http://arxiv.org/pdf/2312.00064v1 are missing entries in their unicode map but do have
                        // entries in the encoding.
                        let encoding = self.encoding.as_ref().map(|x| &x[..]).ok_or(
                            PdfExtractError::Error("missing unicode map and encoding".into()),
                        )?;

                        let s = to_utf8(encoding, &slice)?;

                        trace!("falling back to encoding {} -> {:?}", char, s);

                        s
                    }
                    Some(s) => s.clone(),
                };

            return Ok(s);
        }

        let encoding = self
            .encoding
            .as_ref()
            .map(|x| &x[..])
            .unwrap_or(PDFDocEncoding);

        //trace!("char_code {:?} {:?}", char, self.encoding);

        to_utf8(encoding, &slice)
    }
}

impl<'a> fmt::Debug for PdfSimpleFont<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.font.fmt(f)
    }
}

impl<'a> PdfFont for PdfType3Font<'a> {
    fn get_width(&self, id: CharCode) -> Result<f64> {
        let width = self.widths.get(&id);

        if let Some(width) = width {
            Ok(*width)
        } else {
            Err(PdfExtractError::Error(format!(
                "missing width for {} {:?}",
                id, self.font
            )))?
        }
    }

    /*fn decode(&self, chars: &[u8]) -> String {
        let encoding = self.encoding.as_ref().map(|x| &x[..]).unwrap_or(&PDFDocEncoding);
        to_utf8(encoding, chars)
    }*/

    fn next_char(&self, iter: &mut Iter<u8>) -> Option<(CharCode, u8)> {
        iter.next().map(|x| (*x as CharCode, 1))
    }

    fn decode_char(&self, char: CharCode) -> Result<String> {
        let slice = [char as u8];

        if let Some(ref unicode_map) = self.unicode_map {
            let s = unicode_map.get(&char);

            let s = match s {
                None => Err(PdfExtractError::Error(format!(
                    "missing char {:?} in map {:?}",
                    char, unicode_map
                )))?,
                Some(s) => s.clone(),
            };

            return Ok(s);
        }

        let encoding = self
            .encoding
            .as_ref()
            .map(|x| &x[..])
            .unwrap_or(PDFDocEncoding);
        //trace!("char_code {:?} {:?}", char, self.encoding);

        to_utf8(encoding, &slice)
    }
}

impl<'a> fmt::Debug for PdfType3Font<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.font.fmt(f)
    }
}

struct PdfCIDFont<'a> {
    font: &'a Dictionary,
    doc: &'a Document,
    encoding: ByteMapping,
    to_unicode: Option<HashMap<u32, String>>,
    widths: HashMap<CharCode, f64>, // should probably just use i32 here
    default_width: Option<f64>, // only used for CID fonts and we should probably brake out the different font types
}

fn get_unicode_map<'a>(
    doc: &'a Document,
    font: &'a Dictionary,
) -> Result<Option<HashMap<u32, String>>> {
    let to_unicode = maybe_get_obj(doc, font, b"ToUnicode");

    trace!("ToUnicode: {:?}", to_unicode);

    let mut unicode_map = None;

    match to_unicode {
        Some(Object::Stream(ref stream)) => {
            let contents = get_contents(stream);

            trace!("Stream: {}", String::from_utf8_lossy(&contents));

            let cmap = adobe_cmap_parser::get_unicode_map(&contents).map_err(|error| {
                PdfExtractError::Error(format!("adobe get unicode map failed: {error:?}"))
            })?;

            let mut unicode = HashMap::new();

            // "It must use the beginbfchar, endbfchar, beginbfrange, and endbfrange operators to
            // define the mapping from character codes to Unicode character sequences expressed in
            // UTF-16BE encoding."
            for (&k, v) in cmap.iter() {
                let mut be: Vec<u16> = Vec::new();
                let mut i = 0;

                if v.len() % 2 != 0 {
                    Err(PdfExtractError::Error("length not even".into()))?
                }

                while i < v.len() {
                    be.push(((v[i] as u16) << 8) | v[i + 1] as u16);

                    i += 2;
                }

                if let [0xd800..=0xdfff] = &be[..] {
                    // this range is not specified as not being encoded
                    // we ignore them so we don't an error from from_utt16
                    continue;
                }

                let s = String::from_utf16(&be)?;

                unicode.insert(k, s);
            }

            unicode_map = Some(unicode);

            trace!("map: {:?}", unicode_map);
        }
        None => {}
        Some(Object::Name(ref name)) => {
            let name = pdf_to_utf8(name)?;

            if name != "Identity-H" {
                Err(PdfExtractError::Error(format!(
                    "unsupported ToUnicode name: {:?}",
                    name
                )))?
            }
        }
        _ => Err(PdfExtractError::Error(format!(
            "unsupported cmap {:?}",
            to_unicode
        )))?,
    }

    Ok(unicode_map)
}

impl<'a> PdfCIDFont<'a> {
    fn new(doc: &'a Document, font: &'a Dictionary) -> Result<PdfCIDFont<'a>> {
        let base_name = get_name_string(doc, font, b"BaseFont")?;
        let descendants = maybe_get_array(doc, font, b"DescendantFonts")
            .ok_or(PdfExtractError::Error("Descendant fonts required".into()))?;
        let ciddict = maybe_deref(doc, &descendants[0])?
            .as_dict()
            .map_err(|_| PdfExtractError::Error("should be CID dict".into()))?;
        let encoding = maybe_get_obj(doc, font, b"Encoding").ok_or(PdfExtractError::Error(
            "Encoding required in type0 fonts".into(),
        ))?;

        trace!("base_name {} {:?}", base_name, font);

        let encoding = match encoding {
            Object::Name(ref name) => {
                let name = pdf_to_utf8(name)?;

                trace!("encoding {:?}", name);

                if name != "Identity-H" {
                    Err(PdfExtractError::Error("name is not Identity-H".into()))?
                }

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
            }
            Object::Stream(ref stream) => {
                let contents = get_contents(stream);

                trace!("Stream: {}", String::from_utf8_lossy(&contents));

                adobe_cmap_parser::get_byte_mapping(&contents)
                    .map_err(|value| PdfExtractError::Error(value.to_string()))?
            }
            _ => {
                return Err(PdfExtractError::Error(format!(
                    "unsupported encoding {:?}",
                    encoding
                )))?;
            }
        };

        // Sometimes a Type0 font might refer to the same underlying data as regular font. In this case we may be able to extract some encoding
        // data.
        // We should also look inside the truetype data to see if there's a cmap table. It will help us convert as well.
        // This won't work if the cmap has been subsetted. A better approach might be to hash glyph contents and use that against
        // a global library of glyph hashes
        let unicode_map = get_unicode_map(doc, font)?;

        trace!("descendents {:?} {:?}", descendants, ciddict);

        let Some(font_dict) = maybe_get_obj(doc, ciddict, b"FontDescriptor") else {
            return Err(PdfExtractError::Error("FontDescriptor not found".into()))?;
        };

        trace!("{:?}", font_dict);

        let _f = font_dict
            .as_dict()
            .map_err(|_| PdfExtractError::Error("must be dict".into()))?;
        let default_width = get::<Option<i64>>(doc, ciddict, b"DW")?.unwrap_or(1000);
        let w: Option<Vec<&Object>> = get(doc, ciddict, b"W")?;

        trace!("widths {:?}", w);

        let mut widths = HashMap::new();
        let mut i = 0;

        if let Some(w) = w {
            while i < w.len() {
                if let Object::Array(ref wa) = w[i + 1] {
                    let cid = w[i].as_i64().map_err(|error| {
                        PdfExtractError::Error(format!("id should be num: {error:?}"))
                    })?;

                    trace!("wa: {:?} -> {:?}", cid, wa);

                    for (j, w) in wa.iter().enumerate() {
                        widths.insert((cid as usize + j) as CharCode, as_num(w)?);
                    }

                    i += 2;
                } else {
                    let c_first = w[i].as_i64().map_err(|error| {
                        PdfExtractError::Error(format!("first should be num: {error:?}"))
                    })?;
                    let c_last = w[i].as_i64().map_err(|error| {
                        PdfExtractError::Error(format!("last should be num: {error:?}"))
                    })?;
                    let c_width = as_num(w[i])?;

                    for id in c_first..c_last {
                        widths.insert(id as CharCode, c_width);
                    }

                    i += 3;
                }
            }
        }

        Ok(PdfCIDFont {
            doc,
            font,
            widths,
            to_unicode: unicode_map,
            encoding,
            default_width: Some(default_width as f64),
        })
    }
}

impl<'a> PdfFont for PdfCIDFont<'a> {
    fn get_width(&self, id: CharCode) -> Result<f64> {
        let width = self.widths.get(&id);
        if let Some(width) = width {
            trace!("GetWidth {} -> {}", id, *width);

            Ok(*width)
        } else {
            trace!("missing width for {} falling back to default_width", id);

            Ok(self
                .default_width
                .ok_or(PdfExtractError::Error("no default width".into()))?)
        }
    }

    /*
    fn decode(&self, chars: &[u8]) -> String {
        self.char_codes(chars);

        //let utf16 = Vec::new();

        let encoding = self.encoding.as_ref().map(|x| &x[..]).unwrap_or(&PDFDocEncoding);
        to_utf8(encoding, chars)
    }*/

    fn next_char(&self, iter: &mut Iter<u8>) -> Option<(CharCode, u8)> {
        let mut c = *iter.next()? as u32;
        let mut code = None;

        'outer: for width in 1..=4 {
            for range in &self.encoding.codespace {
                if c >= range.start && c <= range.end && range.width == width {
                    code = Some((c, width));
                    break 'outer;
                }
            }

            let next = *iter.next()?;

            c = (c << 8) | next as u32;
        }

        let code = code?;

        for range in &self.encoding.cid {
            if code.0 >= range.src_code_lo && code.0 <= range.src_code_hi {
                return Some((code.0 + range.dst_CID_lo, code.1 as u8));
            }
        }

        None
    }

    fn decode_char(&self, char: CharCode) -> Result<String> {
        let s = self.to_unicode.as_ref().and_then(|x| x.get(&char));

        let result = if let Some(s) = s {
            s.clone()
        } else {
            trace!(
                "Unknown character {:?} in {:?} {:?}",
                char,
                self.font,
                self.to_unicode
            );

            "".to_string()
        };

        Ok(result)
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
    doc: &'a Document,
}

impl<'a> PdfFontDescriptor<'a> {
    fn get_file(&self) -> Option<&'a Object> {
        maybe_get_obj(self.doc, self.desc, b"FontFile")
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

fn interpolate(x: f64, x_min: f64, _x_max: f64, y_min: f64, y_max: f64) -> f64 {
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
    Type3,
    Type4,
}

impl Function {
    fn new(doc: &Document, obj: &Object) -> Result<Function> {
        let dict = match obj {
            Object::Dictionary(ref dict) => dict,
            Object::Stream(ref stream) => &stream.dict,
            _ => Err(PdfExtractError::Error("unknown object".into()))?,
        };

        let function_type: i64 = get(doc, dict, b"FunctionType")?;

        match function_type {
            0 => {
                let stream = match obj {
                    Object::Stream(ref stream) => stream,
                    _ => Err(PdfExtractError::Error("not a stream".into()))?,
                };

                let range: Vec<f64> = get(doc, dict, b"Range")?;
                let domain: Vec<f64> = get(doc, dict, b"Domain")?;
                let contents = get_contents(stream);
                let size: Vec<i64> = get(doc, dict, b"Size")?;
                let bits_per_sample = get(doc, dict, b"BitsPerSample")?;
                // We ignore 'Order' like pdfium, poppler and pdf.js

                let encode = get::<Option<Vec<f64>>>(doc, dict, b"Encode")?;

                // maybe there's some better way to write this.
                let encode = encode.unwrap_or_else(|| {
                    let mut default = Vec::new();

                    for i in &size {
                        default.extend([0., (i - 1) as f64].iter());
                    }

                    default
                });

                let decode =
                    get::<Option<Vec<f64>>>(doc, dict, b"Decode")?.unwrap_or_else(|| range.clone());

                Ok(Function::Type0(Type0Func {
                    domain,
                    range,
                    size,
                    contents,
                    bits_per_sample,
                    encode,
                    decode,
                }))
            }
            2 => {
                let c0 = get::<Option<Vec<f64>>>(doc, dict, b"C0")?;
                let c1 = get::<Option<Vec<f64>>>(doc, dict, b"C1")?;
                let n = get::<f64>(doc, dict, b"N")?;

                Ok(Function::Type2(Type2Func { c0, c1, n }))
            }
            _ => Err(PdfExtractError::Error(format!(
                "unhandled function type {}",
                function_type
            )))?,
        }
    }
}

fn as_num(o: &Object) -> Result<f64> {
    let result = match o {
        Object::Integer(i) => *i as f64,
        Object::Real(f) => (*f).into(),
        _ => Err(PdfExtractError::Error("not a number: {o:?}".into()))?,
    };

    Ok(result)
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
) -> Result<()> {
    let ts = &mut gs.ts;

    let Some(font) = ts.font.as_ref() else {
        Err(PdfExtractError::Error("font not found".into()))?
    };

    //let encoding = font.encoding.as_ref().map(|x| &x[..]).unwrap_or(&PDFDocEncoding);

    trace!("{:?}", font.decode(s)?);
    trace!("{:?}", font.decode(s)?.as_bytes());
    trace!("{:?}", s);

    output.begin_word()?;

    for (c, length) in font.char_codes(s) {
        // 5.3.3 Text Space Details
        let tsm = Transform2D::row_major(ts.horizontal_scaling, 0., 0., 1.0, 0., ts.rise);

        // Trm = Tsm × Tm × CTM
        let trm = tsm.post_transform(&ts.tm.post_transform(&gs.ctm));

        //trace!("ctm: {:?} tm {:?}", gs.ctm, tm);
        //trace!("current pos: {:?}", position);
        // 5.9 Extraction of Text Content

        //trace!("w: {}", font.widths[&(*c as i64)]);
        let w0 = font.get_width(c)? / 1000.;

        let mut spacing = ts.character_spacing;

        // "Word spacing is applied to every occurrence of the single-byte character code 32 in a
        //  string when using a simple font or a composite font that defines code 32 as a
        //  single-byte code. It does not apply to occurrences of the byte value 32 in
        //  multiple-byte codes."
        let is_space = c == 32 && length == 1;

        if is_space {
            spacing += ts.word_spacing
        }

        output.output_character(&trm, w0, spacing, ts.font_size, &font.decode_char(c)?)?;

        let tj = 0.;
        let ty = 0.;
        let tx = ts.horizontal_scaling * ((w0 - tj / 1000.) * ts.font_size + spacing);

        trace!(
            "horizontal {} adjust {} {} {} {}",
            ts.horizontal_scaling,
            tx,
            w0,
            ts.font_size,
            spacing
        );

        // trace!("w0: {}, tx: {}", w0, tx);

        ts.tm = ts
            .tm
            .pre_transform(&Transform2D::create_translation(tx, ty));
        let _trm = ts.tm.pre_transform(&gs.ctm);

        //trace!("post pos: {:?}", trm);
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

fn apply_state(doc: &Document, gs: &mut GraphicsState, state: &Dictionary) -> Result<()> {
    for (k, v) in state.iter() {
        let k: &[u8] = k.as_ref();

        match k {
            b"SMask" => match maybe_deref(doc, v)? {
                Object::Name(ref name) => {
                    if name == b"None" {
                        gs.smask = None;
                    } else {
                        Err(PdfExtractError::Error("unexpected smask name".into()))?
                    }
                }
                Object::Dictionary(ref dict) => {
                    gs.smask = Some(dict.clone());
                }
                _ => Err(PdfExtractError::Error(format!(
                    "unexpected smask type {:?}",
                    v
                )))?,
            },
            b"Type" => match v {
                Object::Name(ref name) => {
                    if name != b"ExtGState" {
                        Err(PdfExtractError::Error(format!(
                            "{name:?} should be ExtGState"
                        )))?
                    }
                }
                _ => Err(PdfExtractError::Error("unexpected type".into()))?,
            },
            _ => {
                trace!("unapplied state: {:?} {:?}", k, v);
            }
        }
    }

    Ok(())
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
    fn new() -> Path {
        Path { ops: Vec::new() }
    }

    fn current_point(&self) -> Result<(f64, f64)> {
        let result = match self
            .ops
            .last()
            .ok_or(PdfExtractError::Error("no current point".into()))?
        {
            PathOp::MoveTo(x, y) => (*x, *y),
            PathOp::LineTo(x, y) => (*x, *y),
            PathOp::CurveTo(_, _, _, _, x, y) => (*x, *y),
            _ => Err(PdfExtractError::Error("unknown point".into()))?,
        };

        Ok(result)
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
    Pattern,
    CalRGB(CalRGB),
    CalGray(CalGray),
    Lab(Lab),
    Separation(Separation),
    ICCBased(Vec<u8>),
}

fn make_colorspace<'a>(
    doc: &'a Document,
    name: &[u8],
    resources: &'a Dictionary,
) -> Result<ColorSpace> {
    let result = match name {
        b"DeviceGray" => ColorSpace::DeviceGray,
        b"DeviceRGB" => ColorSpace::DeviceRGB,
        b"DeviceCMYK" => ColorSpace::DeviceCMYK,
        b"Pattern" => ColorSpace::Pattern,
        _ => {
            let colorspaces: &Dictionary = get(doc, resources, b"ColorSpace")?;
            let cs: &Object = maybe_get_obj(doc, colorspaces, name).ok_or(
                PdfExtractError::Error(format!("missing colorspace {:?}", name)),
            )?;

            if let Ok(cs) = cs.as_array() {
                let cs_name = pdf_to_utf8(
                    cs[0]
                        .as_name()
                        .map_err(|_| PdfExtractError::Error("first arg must be a name".into()))?,
                )?;

                match cs_name.as_ref() {
                    "Separation" => {
                        let name = pdf_to_utf8(cs[1].as_name().map_err(|_| {
                            PdfExtractError::Error("second arg must be a name".into())
                        })?)?;

                        let alternate_space = match &maybe_deref(doc, &cs[2])? {
                            Object::Name(name) => match &name[..] {
                                b"DeviceGray" => AlternateColorSpace::DeviceGray,
                                b"DeviceRGB" => AlternateColorSpace::DeviceRGB,
                                b"DeviceCMYK" => AlternateColorSpace::DeviceCMYK,
                                _ => Err(PdfExtractError::Error(format!(
                                    "Unexpected color space name {name:?}",
                                )))?,
                            },
                            Object::Array(cs) => {
                                let cs_name = pdf_to_utf8(cs[0].as_name().map_err(|_| {
                                    PdfExtractError::Error("first arg must be a name".into())
                                })?)?;

                                match cs_name.as_ref() {
                                    "ICCBased" => {
                                        let stream = maybe_deref(doc, &cs[1])?.as_stream()?;

                                        trace!("ICCBased {:?}", stream);

                                        // XXX: we're going to be continually decompressing everytime this object is referenced
                                        AlternateColorSpace::ICCBased(get_contents(stream))
                                    }
                                    "CalGray" => {
                                        let dict = cs[1].as_dict().map_err(|_| {
                                            PdfExtractError::Error(
                                                "second arg must be a dict".into(),
                                            )
                                        })?;

                                        AlternateColorSpace::CalGray(CalGray {
                                            white_point: get(doc, dict, b"WhitePoint")?,
                                            black_point: get(doc, dict, b"BackPoint")?,
                                            gamma: get(doc, dict, b"Gamma")?,
                                        })
                                    }
                                    "CalRGB" => {
                                        let dict = cs[1].as_dict().map_err(|_| {
                                            PdfExtractError::Error(
                                                "second arg must be a dict".into(),
                                            )
                                        })?;

                                        AlternateColorSpace::CalRGB(CalRGB {
                                            white_point: get(doc, dict, b"WhitePoint")?,
                                            black_point: get(doc, dict, b"BackPoint")?,
                                            gamma: get(doc, dict, b"Gamma")?,
                                            matrix: get(doc, dict, b"Matrix")?,
                                        })
                                    }
                                    "Lab" => {
                                        let dict = cs[1].as_dict().map_err(|_| {
                                            PdfExtractError::Error(
                                                "second arg must be a dict".into(),
                                            )
                                        })?;

                                        AlternateColorSpace::Lab(Lab {
                                            white_point: get(doc, dict, b"WhitePoint")?,
                                            black_point: get(doc, dict, b"BackPoint")?,
                                            range: get(doc, dict, b"Range")?,
                                        })
                                    }
                                    _ => Err(PdfExtractError::Error(format!(
                                        "Unexpected color space name {cs_name}",
                                    )))?,
                                }
                            }
                            _ => Err(PdfExtractError::Error(format!(
                                "Alternate space should be name or array {:?}",
                                cs[2]
                            )))?,
                        };

                        let tint_transform =
                            Box::new(Function::new(doc, maybe_deref(doc, &cs[3])?)?);

                        trace!("{:?} {:?} {:?}", name, alternate_space, tint_transform);

                        ColorSpace::Separation(Separation {
                            name,
                            alternate_space,
                            tint_transform,
                        })
                    }
                    "ICCBased" => {
                        let stream = maybe_deref(doc, &cs[1])?.as_stream()?;

                        trace!("ICCBased {:?}", stream);

                        // XXX: we're going to be continually decompressing everytime this object is referenced
                        ColorSpace::ICCBased(get_contents(stream))
                    }
                    "CalGray" => {
                        let dict = cs[1].as_dict().map_err(|_| {
                            PdfExtractError::Error("second arg must be a dict".into())
                        })?;

                        ColorSpace::CalGray(CalGray {
                            white_point: get(doc, dict, b"WhitePoint")?,
                            black_point: get(doc, dict, b"BackPoint")?,
                            gamma: get(doc, dict, b"Gamma")?,
                        })
                    }
                    "CalRGB" => {
                        let dict = cs[1].as_dict().map_err(|_| {
                            PdfExtractError::Error("second arg must be a dict".into())
                        })?;

                        ColorSpace::CalRGB(CalRGB {
                            white_point: get(doc, dict, b"WhitePoint")?,
                            black_point: get(doc, dict, b"BackPoint")?,
                            gamma: get(doc, dict, b"Gamma")?,
                            matrix: get(doc, dict, b"Matrix")?,
                        })
                    }
                    "Lab" => {
                        let dict = cs[1].as_dict().map_err(|_| {
                            PdfExtractError::Error("second arg must be a dict".into())
                        })?;

                        ColorSpace::Lab(Lab {
                            white_point: get(doc, dict, b"WhitePoint")?,
                            black_point: get(doc, dict, b"BackPoint")?,
                            range: get(doc, dict, b"Range")?,
                        })
                    }
                    "Pattern" => ColorSpace::Pattern,
                    "DeviceGray" => ColorSpace::DeviceGray,
                    "DeviceRGB" => ColorSpace::DeviceRGB,
                    "DeviceCMYK" => ColorSpace::DeviceCMYK,
                    _ => Err(PdfExtractError::Error(format!(
                        "color_space {:?} {:?} {:?}",
                        name, cs_name, cs
                    )))?,
                }
            } else if let Ok(cs) = cs.as_name() {
                let name = pdf_to_utf8(cs)?;

                match name.as_ref() {
                    "DeviceRGB" => ColorSpace::DeviceRGB,
                    "DeviceGray" => ColorSpace::DeviceGray,
                    _ => Err(PdfExtractError::Error(format!(
                        "unknown color space device {name:?}"
                    )))?,
                }
            } else {
                Err(PdfExtractError::Error(format!(
                    "unknown color space {name:?}"
                )))?
            }
        }
    };

    Ok(result)
}

struct Processor {}

impl Processor {
    fn process_stream(
        doc: &Document,
        content: Vec<u8>,
        resources: &Dictionary,
        media_box: &MediaBox,
        output: &mut dyn OutputDev,
        _page_num: u32,
    ) -> Result<()> {
        let content = Content::decode(&content)?;

        let mut font_table: HashMap<String, Rc<dyn PdfFont>> = HashMap::new();

        let mut gs: GraphicsState = GraphicsState {
            ts: TextState {
                font: None,
                font_size: std::f64::NAN,
                character_spacing: 0.,
                word_spacing: 0.,
                horizontal_scaling: 1.,
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

        trace!("MediaBox {:?}", media_box);

        for operation in &content.operations {
            //trace!("op: {:?}", operation);

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
                    if operation.operands.len() != 6 {
                        return Err(PdfExtractError::Error("cm has wrong size".into()))?;
                    }

                    let m = Transform2D::row_major(
                        as_num(&operation.operands[0])?,
                        as_num(&operation.operands[1])?,
                        as_num(&operation.operands[2])?,
                        as_num(&operation.operands[3])?,
                        as_num(&operation.operands[4])?,
                        as_num(&operation.operands[5])?,
                    );

                    gs.ctm = gs.ctm.pre_transform(&m);

                    trace!("matrix {:?}", gs.ctm);
                }
                "CS" => {
                    let name = operation.operands[0].as_name()?;

                    gs.stroke_colorspace = make_colorspace(doc, name, resources)?;
                }
                "cs" => {
                    let name = operation.operands[0].as_name()?;

                    gs.fill_colorspace = make_colorspace(doc, name, resources)?;
                }
                "SC" | "SCN" => {
                    gs.stroke_color = match gs.stroke_colorspace {
                        ColorSpace::Pattern => {
                            trace!("unhandled pattern color");

                            Vec::new()
                        }
                        _ => operation
                            .operands
                            .iter()
                            .map(as_num)
                            .collect::<Result<Vec<_>>>()?,
                    };
                }
                "sc" | "scn" => {
                    gs.fill_color = match gs.fill_colorspace {
                        ColorSpace::Pattern => {
                            trace!("unhandled pattern color");

                            Vec::new()
                        }
                        _ => operation
                            .operands
                            .iter()
                            .map(as_num)
                            .collect::<Result<Vec<_>>>()?,
                    };
                }
                "G" | "g" | "RG" | "rg" | "K" | "k" => {
                    trace!("unhandled color operation {:?}", operation);
                }
                "TJ" => {
                    if let Object::Array(ref array) = operation.operands[0] {
                        for e in array {
                            match e {
                                Object::String(ref s, _) => {
                                    show_text(&mut gs, s, &tlm, &flip_ctm, output)?;
                                }
                                Object::Integer(i) => {
                                    let ts = &mut gs.ts;
                                    let w0 = 0.;
                                    let tj = *i as f64;
                                    let ty = 0.;
                                    let tx =
                                        ts.horizontal_scaling * ((w0 - tj / 1000.) * ts.font_size);
                                    ts.tm = ts
                                        .tm
                                        .pre_transform(&Transform2D::create_translation(tx, ty));

                                    trace!("adjust text by: {} {:?}", i, ts.tm);
                                }
                                Object::Real(i) => {
                                    let ts = &mut gs.ts;
                                    let w0 = 0.;
                                    let tj = *i as f64;
                                    let ty = 0.;
                                    let tx =
                                        ts.horizontal_scaling * ((w0 - tj / 1000.) * ts.font_size);
                                    ts.tm = ts
                                        .tm
                                        .pre_transform(&Transform2D::create_translation(tx, ty));

                                    trace!("adjust text by: {} {:?}", i, ts.tm);
                                }
                                _ => {
                                    trace!("kind of {:?}", e);
                                }
                            }
                        }
                    }
                }
                "Tj" => match operation.operands[0] {
                    Object::String(ref s, _) => {
                        show_text(&mut gs, s, &tlm, &flip_ctm, output)?;
                    }
                    _ => Err(PdfExtractError::Error(format!(
                        "unexpected Tj operand {:?}",
                        operation
                    )))?,
                },
                "Tc" => {
                    gs.ts.character_spacing = as_num(&operation.operands[0])?;
                }
                "Tw" => {
                    gs.ts.word_spacing = as_num(&operation.operands[0])?;
                }
                "Tz" => {
                    gs.ts.horizontal_scaling = as_num(&operation.operands[0])? / 100.;
                }
                "TL" => {
                    gs.ts.leading = as_num(&operation.operands[0])?;
                }
                "Tf" => {
                    let fonts: &Dictionary = get(doc, resources, b"Font")?;
                    let name =
                        String::from_utf8_lossy(operation.operands[0].as_name()?).to_string();

                    let font: Rc<dyn PdfFont> = match font_table.get(&name) {
                        Some(value) => value.to_owned(),
                        None => {
                            let font =
                                make_font(doc, get::<&Dictionary>(doc, fonts, name.as_bytes())?)?;

                            font_table.insert(name.to_string(), font.clone());

                            font
                        }
                    };

                    /*
                    {
                        let file = font.get_descriptor().and_then(|desc| desc.get_file());

                        if let Some(file) = file {
                            let file_contents = filter_data(file.as_stream().unwrap());

                            let mut cursor = Cursor::new(&file_contents[..]);

                            //let f = Font::read(&mut cursor);

                            //trace!("font file: {:?}", f);
                        }
                    }
                    */

                    gs.ts.font = Some(font);

                    gs.ts.font_size = as_num(&operation.operands[1])?;

                    trace!(
                        "font {} size: {} {:?}",
                        pdf_to_utf8(name.as_bytes())?,
                        gs.ts.font_size,
                        operation
                    );
                }
                "Ts" => {
                    gs.ts.rise = as_num(&operation.operands[0])?;
                }
                "Tm" => {
                    if operation.operands.len() != 6 {
                        Err(PdfExtractError::Error("operation length is not 6".into()))?
                    }

                    tlm = Transform2D::row_major(
                        as_num(&operation.operands[0])?,
                        as_num(&operation.operands[1])?,
                        as_num(&operation.operands[2])?,
                        as_num(&operation.operands[3])?,
                        as_num(&operation.operands[4])?,
                        as_num(&operation.operands[5])?,
                    );
                    gs.ts.tm = tlm;

                    trace!("Tm: matrix {:?}", gs.ts.tm);

                    output.end_line()?;
                }
                "Td" => {
                    /* Move to the start of the next line, offset from the start of the current line by (tx , ty ).
                      tx and ty are numbers expressed in unscaled text space units.
                      More precisely, this operator performs the following assignments:
                    */
                    if operation.operands.len() != 2 {
                        Err(PdfExtractError::Error("operation length is not 2".into()))?
                    }

                    let tx = as_num(&operation.operands[0])?;
                    let ty = as_num(&operation.operands[1])?;

                    trace!("translation: {} {}", tx, ty);

                    tlm = tlm.pre_transform(&Transform2D::create_translation(tx, ty));
                    gs.ts.tm = tlm;

                    trace!("Td matrix {:?}", gs.ts.tm);

                    output.end_line()?;
                }

                "TD" => {
                    /* Move to the start of the next line, offset from the start of the current line by (tx , ty ).
                      As a side effect, this operator sets the leading parameter in the text state.
                    */
                    if operation.operands.len() != 2 {
                        Err(PdfExtractError::Error("operation length is not 2".into()))?
                    }

                    let tx = as_num(&operation.operands[0])?;
                    let ty = as_num(&operation.operands[1])?;

                    trace!("translation: {} {}", tx, ty);

                    gs.ts.leading = -ty;

                    tlm = tlm.pre_transform(&Transform2D::create_translation(tx, ty));
                    gs.ts.tm = tlm;

                    trace!("TD matrix {:?}", gs.ts.tm);

                    output.end_line()?;
                }

                "T*" => {
                    let tx = 0.0;
                    let ty = -gs.ts.leading;

                    tlm = tlm.pre_transform(&Transform2D::create_translation(tx, ty));
                    gs.ts.tm = tlm;

                    trace!("T* matrix {:?}", gs.ts.tm);

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
                        trace!("No state to pop");
                    }
                }
                "gs" => {
                    let ext_gstate: &Dictionary = get(doc, resources, b"ExtGState")?;
                    let name = operation.operands[0].as_name()?;
                    let state: &Dictionary = get(doc, ext_gstate, name)?;

                    apply_state(doc, &mut gs, state)?;
                }
                "i" => {
                    trace!(
                        "unhandled graphics state flattness operator {:?}",
                        operation
                    );
                }
                "w" => {
                    gs.line_width = as_num(&operation.operands[0])?;
                }
                "J" | "j" | "M" | "d" | "ri" => {
                    trace!("unknown graphics state operator {:?}", operation);
                }
                "m" => path.ops.push(PathOp::MoveTo(
                    as_num(&operation.operands[0])?,
                    as_num(&operation.operands[1])?,
                )),
                "l" => path.ops.push(PathOp::LineTo(
                    as_num(&operation.operands[0])?,
                    as_num(&operation.operands[1])?,
                )),
                "c" => path.ops.push(PathOp::CurveTo(
                    as_num(&operation.operands[0])?,
                    as_num(&operation.operands[1])?,
                    as_num(&operation.operands[2])?,
                    as_num(&operation.operands[3])?,
                    as_num(&operation.operands[4])?,
                    as_num(&operation.operands[5])?,
                )),
                "v" => {
                    let (x, y) = path.current_point()?;

                    path.ops.push(PathOp::CurveTo(
                        x,
                        y,
                        as_num(&operation.operands[0])?,
                        as_num(&operation.operands[1])?,
                        as_num(&operation.operands[2])?,
                        as_num(&operation.operands[3])?,
                    ))
                }
                "y" => path.ops.push(PathOp::CurveTo(
                    as_num(&operation.operands[0])?,
                    as_num(&operation.operands[1])?,
                    as_num(&operation.operands[2])?,
                    as_num(&operation.operands[3])?,
                    as_num(&operation.operands[2])?,
                    as_num(&operation.operands[3])?,
                )),
                "h" => path.ops.push(PathOp::Close),
                "re" => path.ops.push(PathOp::Rect(
                    as_num(&operation.operands[0])?,
                    as_num(&operation.operands[1])?,
                    as_num(&operation.operands[2])?,
                    as_num(&operation.operands[3])?,
                )),
                "s" | "f*" | "B" | "B*" | "b" => {
                    trace!("unhandled path op {:?}", operation);
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
                    trace!("unhandled clipping operation {:?}", operation);
                }
                "n" => {
                    trace!("discard {:?}", path);

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
                    let xobject: &Dictionary = get(doc, resources, b"XObject")?;
                    let name = operation.operands[0].as_name()?;
                    let xf: &Stream = get(doc, xobject, name)?;
                    let resources = maybe_get_obj(doc, &xf.dict, b"Resources")
                        .and_then(|n| n.as_dict().ok())
                        .unwrap_or(resources);
                    let contents = get_contents(xf);

                    Processor::process_stream(
                        doc, contents, resources, media_box, output, _page_num,
                    )?;
                }
                _ => {
                    trace!("unknown operation {:?}", operation);
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
    ) -> Result<()>;

    fn end_page(&mut self) -> Result<()>;

    fn output_character(
        &mut self,
        trm: &Transform,
        width: f64,
        spacing: f64,
        font_size: f64,
        char: &str,
    ) -> Result<()>;

    fn begin_word(&mut self) -> Result<()>;

    fn end_word(&mut self) -> Result<()>;

    fn end_line(&mut self) -> Result<()>;

    fn stroke(
        &mut self,
        _ctm: &Transform,
        _colorspace: &ColorSpace,
        _color: &[f64],
        _path: &Path,
    ) -> Result<()> {
        Ok(())
    }

    fn fill(
        &mut self,
        _ctm: &Transform,
        _colorspace: &ColorSpace,
        _color: &[f64],
        _path: &Path,
    ) -> Result<()> {
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

    fn flush_string(&mut self) -> Result<()> {
        if !self.buf.is_empty() {
            let position = self.buf_ctm.post_transform(&self.flip_ctm);
            let transformed_font_size_vec = self
                .buf_ctm
                .transform_vector(vec2(self.buf_font_size, self.buf_font_size));

            // get the length of one sized of the square with the same area with a rectangle of size (x, y)
            let transformed_font_size =
                (transformed_font_size_vec.x * transformed_font_size_vec.y).sqrt();
            let (x, y) = (position.m31, position.m32);

            trace!("flush {} {:?}", self.buf, (x, y));

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

impl<'a> OutputDev for HTMLOutput<'a> {
    fn begin_page(&mut self, page_num: u32, media_box: &MediaBox, _: Option<ArtBox>) -> Result<()> {
        write!(self.file, "<meta charset='utf-8' /> ")?;
        write!(self.file, "<!-- page {} -->", page_num)?;
        write!(self.file, "<div id='page{}' style='position: relative; height: {}px; width: {}px; border: 1px black solid'>", page_num, media_box.ury - media_box.lly, media_box.urx - media_box.llx)?;

        self.flip_ctm = Transform::row_major(1., 0., 0., -1., 0., media_box.ury - media_box.lly);

        Ok(())
    }

    fn end_page(&mut self) -> Result<()> {
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
    ) -> Result<()> {
        if trm.approx_eq(&self.last_ctm) {
            let position = trm.post_transform(&self.flip_ctm);
            let (x, y) = (position.m31, position.m32);

            trace!("accum {} {:?}", char, (x, y));

            self.buf += char;
        } else {
            trace!(
                "flush {} {:?} {:?} {} {} {}",
                char,
                trm,
                self.last_ctm,
                width,
                font_size,
                spacing
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

        write!(self.file, "<div style='position: absolute; color: red; left: {}px; top: {}px; font-size: {}px'>{}</div>",
               x, y, transformed_font_size, char)?;

        self.last_ctm = trm.pre_transform(&Transform2D::create_translation(
            width * font_size + spacing,
            0.,
        ));

        Ok(())
    }

    fn begin_word(&mut self) -> Result<()> {
        Ok(())
    }

    fn end_word(&mut self) -> Result<()> {
        Ok(())
    }

    fn end_line(&mut self) -> Result<()> {
        Ok(())
    }
}

pub struct SVGOutput<'a> {
    file: &'a mut dyn std::io::Write,
}

impl<'a> SVGOutput<'a> {
    pub fn new(file: &mut dyn std::io::Write) -> SVGOutput {
        SVGOutput { file }
    }
}

impl<'a> OutputDev for SVGOutput<'a> {
    fn begin_page(
        &mut self,
        _page_num: u32,
        media_box: &MediaBox,
        art_box: Option<(f64, f64, f64, f64)>,
    ) -> Result<()> {
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

    fn end_page(&mut self) -> Result<()> {
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
    ) -> Result<()> {
        Ok(())
    }

    fn begin_word(&mut self) -> Result<()> {
        Ok(())
    }

    fn end_word(&mut self) -> Result<()> {
        Ok(())
    }

    fn end_line(&mut self) -> Result<()> {
        Ok(())
    }

    fn fill(
        &mut self,
        ctm: &Transform,
        _colorspace: &ColorSpace,
        _color: &[f64],
        path: &Path,
    ) -> Result<()> {
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
            match op {
                PathOp::MoveTo(x, y) => d.push(format!("M{} {}", x, y)),
                PathOp::LineTo(x, y) => d.push(format!("L{} {}", x, y)),
                PathOp::CurveTo(x1, y1, x2, y2, x, y) => {
                    d.push(format!("C{} {} {} {} {} {}", x1, y1, x2, y2, x, y))
                }
                PathOp::Close => d.push("Z".into()),
                PathOp::Rect(x, y, width, height) => {
                    d.push(format!("M{} {}", x, y));
                    d.push(format!("L{} {}", x + width, y));
                    d.push(format!("L{} {}", x + width, y + height));
                    d.push(format!("L{} {}", x, y + height));
                    d.push("Z".into());
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

pub struct PlainTextOutput<W: ConvertToFmt> {
    writer: W::Writer,
    last_end: f64,
    last_y: f64,
    first_char: bool,
    flip_ctm: Transform,
}

impl<W: ConvertToFmt> PlainTextOutput<W> {
    pub fn new(writer: W) -> PlainTextOutput<W> {
        PlainTextOutput {
            writer: writer.convert(),
            last_end: 100000.,
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
    ) -> Result<()> {
        self.flip_ctm = Transform2D::row_major(1., 0., 0., -1., 0., media_box.ury - media_box.lly);

        Ok(())
    }

    fn end_page(&mut self) -> Result<()> {
        Ok(())
    }

    fn output_character(
        &mut self,
        trm: &Transform,
        width: f64,
        _spacing: f64,
        font_size: f64,
        char: &str,
    ) -> Result<()> {
        let position = trm.post_transform(&self.flip_ctm);
        let transformed_font_size_vec = trm.transform_vector(vec2(font_size, font_size));

        // get the length of one sized of the square with the same area with a rectangle of size (x, y)
        let transformed_font_size =
            (transformed_font_size_vec.x * transformed_font_size_vec.y).sqrt();
        let (x, y) = (position.m31, position.m32);

        //trace!("last_end: {} x: {}, width: {}", self.last_end, x, width);
        if self.first_char {
            if (y - self.last_y).abs() > transformed_font_size * 1.5 {
                writeln!(self.writer)?;
            }

            // we've moved to the left and down
            if x < self.last_end && (y - self.last_y).abs() > transformed_font_size * 0.5 {
                writeln!(self.writer)?;
            }

            if x > self.last_end + transformed_font_size * 0.1 {
                trace!(
                    "width: {}, space: {}, thresh: {}",
                    width,
                    x - self.last_end,
                    transformed_font_size * 0.1
                );

                write!(self.writer, " ")?;
            }
        }

        //let norm = unicode_normalization::UnicodeNormalization::nfkc(char);
        write!(self.writer, "{}", char)?;

        self.first_char = false;
        self.last_y = y;
        self.last_end = x + width * transformed_font_size;

        Ok(())
    }

    fn begin_word(&mut self) -> Result<()> {
        self.first_char = true;
        Ok(())
    }

    fn end_word(&mut self) -> Result<()> {
        Ok(())
    }

    fn end_line(&mut self) -> Result<()> {
        //write!(self.file, "\n");

        Ok(())
    }
}

pub fn print_metadata(doc: &Document) -> Result<()> {
    trace!("Version: {}", doc.version);

    if let Some(info) = get_info(doc) {
        for (k, v) in info {
            if let &Object::String(ref s, StringFormat::Literal) = v {
                trace!("{}: {}", pdf_to_utf8(k)?, pdf_to_utf8(s)?);
            }
        }
    }

    trace!(
        "Page count: {}",
        get::<i64>(doc, get_pages(doc)?, b"Count")?
    );

    trace!("Pages: {:?}", get_pages(doc));

    trace!(
        "Type: {:?}",
        get_pages(doc)?.get(b"Type").and_then(|x| x.as_name())?
    );

    Ok(())
}

/// Extract the text from a pdf at `path` and return a `String` with the results
pub fn extract_text<P: std::convert::AsRef<std::path::Path>>(path: P) -> Result<String> {
    let mut s = String::new();

    {
        let mut output = PlainTextOutput::new(&mut s);
        let mut doc = Document::load(path)?;

        maybe_decrypt(&mut doc)?;

        output_doc(&doc, &mut output)?;
    }

    Ok(s)
}

fn maybe_decrypt(doc: &mut Document) -> Result<()> {
    if !doc.is_encrypted() {
        return Ok(());
    }

    if let Err(e) = doc.decrypt("") {
        if let Error::Decryption(DecryptionError::IncorrectPassword) = e {
            error!("Encrypted documents must be decrypted with a password using {{extract_text|extract_text_from_mem|output_doc}}_encrypted")
        }

        return Err(PdfExtractError::PdfError(e).into());
    }

    Ok(())
}

pub fn extract_text_encrypted<P: std::convert::AsRef<std::path::Path>, PW: AsRef<[u8]>>(
    path: P,
    password: PW,
) -> Result<String> {
    let mut s = String::new();
    {
        let mut output = PlainTextOutput::new(&mut s);
        let mut doc = Document::load(path)?;

        output_doc_encrypted(&mut doc, &mut output, password)?;
    }
    Ok(s)
}

pub fn extract_text_from_mem(buffer: &[u8]) -> Result<String> {
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
) -> Result<String> {
    let mut s = String::new();
    {
        let mut output = PlainTextOutput::new(&mut s);
        let mut doc = Document::load_mem(buffer)?;

        output_doc_encrypted(&mut doc, &mut output, password)?;
    }
    Ok(s)
}

fn get_inherited<'a, T: FromObj<'a>>(
    doc: &'a Document,
    dict: &'a Dictionary,
    key: &[u8],
) -> Result<Option<T>> {
    let o: Option<T> = get(doc, dict, key)?;

    if let Some(o) = o {
        Ok(Some(o))
    } else {
        let parent = dict
            .get(b"Parent")
            .and_then(|parent| parent.as_reference())
            .and_then(|id| doc.get_dictionary(id))
            .ok()
            .ok_or(PdfExtractError::Error("no parent found".to_string()))?;

        get_inherited(doc, parent, key)
    }
}

pub fn output_doc_encrypted<PW: AsRef<[u8]>>(
    doc: &mut Document,
    output: &mut dyn OutputDev,
    password: PW,
) -> Result<()> {
    doc.decrypt(password)?;

    output_doc(doc, output)
}

/// Parse a given document and output it to `output`
pub fn output_doc(doc: &Document, output: &mut dyn OutputDev) -> Result<()> {
    if doc.is_encrypted() {
        error!("Encrypted documents must be decrypted with a password using {{extract_text|extract_text_from_mem|output_doc}}_encrypted");

        return Err(PdfExtractError::Encrypted.into());
    }

    let empty_resources = &Dictionary::new();
    let pages = doc.get_pages();

    for dict in pages {
        let page_num = dict.0;
        let page_dict = doc.get_object(dict.1)?.as_dict()?;

        debug!("page {} {:?}", page_num, page_dict);

        // XXX: Some pdfs lack a Resources directory
        let resources = get_inherited(doc, page_dict, b"Resources")?.unwrap_or(empty_resources);

        debug!("resources {:?}", resources);

        // pdfium searches up the page tree for MediaBoxes as needed
        let media_box: Vec<f64> = get_inherited(doc, page_dict, b"MediaBox")?
            .ok_or(PdfExtractError::Error("MediaBox".into()))?;

        let media_box = MediaBox {
            llx: media_box[0],
            lly: media_box[1],
            urx: media_box[2],
            ury: media_box[3],
        };

        let art_box =
            get::<Option<Vec<f64>>>(doc, page_dict, b"ArtBox")?.map(|x| (x[0], x[1], x[2], x[3]));

        output.begin_page(page_num, &media_box, art_box)?;

        Processor::process_stream(
            doc,
            doc.get_page_content(dict.1)?,
            resources,
            &media_box,
            output,
            page_num,
        )?;

        output.end_page()?;
    }

    Ok(())
}
