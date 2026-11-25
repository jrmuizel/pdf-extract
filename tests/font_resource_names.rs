use lopdf::content::{Content, Operation};
use lopdf::{dictionary, Document, Object, Stream};
use pdf_extract::extract_text_from_mem;

// Font resource names are only unique within one resource dictionary. A form
// XObject with its own /Resources may reuse /F1 for a different font than the
// page does, and the text of each must be decoded with its own font.

fn font(doc: &mut Document, base: &str) -> Object {
    doc.add_object(dictionary! {
        "Type" => "Font", "Subtype" => "Type1", "BaseFont" => base,
    })
    .into()
}

fn show(name: &str, text: &str) -> Vec<Operation> {
    vec![
        Operation::new("BT", vec![]),
        Operation::new("Tf", vec![Object::Name(name.as_bytes().to_vec()), 12.into()]),
        Operation::new("Td", vec![72.into(), 700.into()]),
        Operation::new("Tj", vec![Object::string_literal(text)]),
        Operation::new("ET", vec![]),
    ]
}

fn form(doc: &mut Document, base: &str, text: &str) -> Object {
    let f1 = font(doc, base);
    let content = Content { operations: show("F1", text) };
    doc.add_object(Stream::new(
        dictionary! {
            "Type" => "XObject", "Subtype" => "Form",
            "BBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
            "Resources" => dictionary! { "Font" => dictionary! { "F1" => f1 } },
        },
        content.encode().unwrap(),
    ))
    .into()
}

// `steps` is the page content in order: ("text", ...) draws with the page's own
// /F1, ("form", n) paints form n.
fn build(page_font: &str, forms: &[(&str, &str)], steps: &[&str]) -> Vec<u8> {
    let mut doc = Document::with_version("1.5");
    let pages_id = doc.new_object_id();
    let page_f1 = font(&mut doc, page_font);
    let mut xobjects = lopdf::Dictionary::new();
    for (i, (base, text)) in forms.iter().enumerate() {
        let f = form(&mut doc, base, text);
        xobjects.set(format!("Fm{}", i), f);
    }
    let mut ops = Vec::new();
    for step in steps {
        match *step {
            "text" => ops.extend(show("F1", "abg")),
            form => ops.push(Operation::new("Do", vec![Object::Name(form.as_bytes().to_vec())])),
        }
    }
    let content_id = doc.add_object(Stream::new(dictionary! {}, Content { operations: ops }.encode().unwrap()));
    let page_id = doc.add_object(dictionary! {
        "Type" => "Page", "Parent" => pages_id,
        "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
        "Contents" => content_id,
        "Resources" => dictionary! {
            "Font" => dictionary! { "F1" => page_f1 },
            "XObject" => xobjects,
        },
    });
    doc.objects.insert(pages_id, dictionary! {
        "Type" => "Pages", "Kids" => vec![page_id.into()], "Count" => 1,
    }.into());
    let catalog = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
    doc.trailer.set("Root", catalog);
    let mut out = Vec::new();
    doc.save_to(&mut out).unwrap();
    out
}

fn text(pdf: Vec<u8>) -> String {
    extract_text_from_mem(&pdf).unwrap()
}

// Symbol maps the bytes `abg` to Greek letters, Helvetica leaves them as is.

#[test]
fn form_font_after_page_font_with_the_same_name() {
    let out = text(build("Symbol", &[("Helvetica", "abg")], &["text", "Fm0"]));
    assert!(out.contains("αβγ") && out.contains("abg"), "got {:?}", out);
}

#[test]
fn form_font_before_page_font_with_the_same_name() {
    let out = text(build("Symbol", &[("Helvetica", "abg")], &["Fm0", "text"]));
    assert!(out.contains("αβγ") && out.contains("abg"), "got {:?}", out);
}

#[test]
fn two_forms_reusing_one_font_name() {
    let out = text(build("Helvetica", &[("Symbol", "abg"), ("Helvetica", "abg")], &["Fm0", "Fm1"]));
    assert!(out.contains("αβγ") && out.contains("abg"), "got {:?}", out);
}
