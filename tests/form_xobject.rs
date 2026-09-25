use lopdf::content::{Content, Operation};
use lopdf::{dictionary, Document, Object, Stream};
use pdf_extract::{output_doc, MediaBox, OutputDev, OutputError, Transform};

/// Records the device-space origin of every character.
struct Positions(Vec<(String, f64, f64)>);

impl OutputDev for Positions {
    fn begin_page(&mut self, _: u32, _: &MediaBox, _: Option<(f64, f64, f64, f64)>) -> Result<(), OutputError> { Ok(()) }
    fn end_page(&mut self) -> Result<(), OutputError> { Ok(()) }
    fn output_character(&mut self, trm: &Transform, _: f64, _: f64, _: f64, char: &str) -> Result<(), OutputError> {
        self.0.push((char.to_string(), trm.m31, trm.m32));
        Ok(())
    }
    fn begin_word(&mut self) -> Result<(), OutputError> { Ok(()) }
    fn end_word(&mut self) -> Result<(), OutputError> { Ok(()) }
    fn end_line(&mut self) -> Result<(), OutputError> { Ok(()) }
}

fn op(name: &str, operands: Vec<Object>) -> Operation {
    Operation::new(name, operands)
}

// A page that draws a form XObject (with a non-identity /Matrix) under a
// translated CTM, and an image XObject whose raw bytes happen to look like a
// content stream.
fn build_doc() -> Document {
    let mut doc = Document::with_version("1.5");
    let pages_id = doc.new_object_id();
    let font_id = doc.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica",
    });
    let resources = dictionary! { "Font" => dictionary! { "F1" => font_id } };

    let form_content = Content { operations: vec![
        op("BT", vec![]),
        op("Tf", vec!["F1".into(), 12.into()]),
        op("Td", vec![10.into(), 20.into()]),
        op("Tj", vec![Object::string_literal("A")]),
        op("ET", vec![]),
    ]};
    let form_id = doc.add_object(Stream::new(dictionary! {
        "Type" => "XObject",
        "Subtype" => "Form",
        "BBox" => vec![0.into(), 0.into(), 100.into(), 100.into()],
        "Matrix" => vec![2.into(), 0.into(), 0.into(), 2.into(), 5.into(), 7.into()],
        "Resources" => resources.clone(),
    }, form_content.encode().unwrap()));

    let image_id = doc.add_object(Stream::new(dictionary! {
        "Type" => "XObject",
        "Subtype" => "Image",
        "Width" => 1,
        "Height" => 1,
        "ColorSpace" => "DeviceGray",
        "BitsPerComponent" => 8,
    }, b"BT /F1 12 Tf (B) Tj ET".to_vec()));

    let page_content = Content { operations: vec![
        op("q", vec![]),
        op("cm", vec![1.into(), 0.into(), 0.into(), 1.into(), 100.into(), 200.into()]),
        op("Do", vec!["Fm0".into()]),
        op("Q", vec![]),
        op("Do", vec!["Im0".into()]),
    ]};
    let content_id = doc.add_object(Stream::new(dictionary! {}, page_content.encode().unwrap()));
    let mut page_resources = resources;
    page_resources.set("XObject", dictionary! { "Fm0" => form_id, "Im0" => image_id });
    let page_id = doc.add_object(dictionary! {
        "Type" => "Page",
        "Parent" => pages_id,
        "Contents" => content_id,
        "Resources" => page_resources,
        "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
    });
    doc.objects.insert(pages_id, Object::Dictionary(dictionary! {
        "Type" => "Pages",
        "Kids" => vec![page_id.into()],
        "Count" => 1,
    }));
    let catalog_id = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
    doc.trailer.set("Root", catalog_id);
    doc
}

#[test]
fn form_xobject_uses_invoking_ctm_and_matrix() {
    let doc = build_doc();
    let mut out = Positions(Vec::new());
    output_doc(&doc, &mut out).unwrap();

    // Only the form's text is drawn; the image data is not interpreted as content.
    assert_eq!(out.0.len(), 1, "unexpected characters: {:?}", out.0);
    let (ref c, x, y) = out.0[0];
    assert_eq!(c, "A");
    // Td (10, 20) mapped through /Matrix [2 0 0 2 5 7] and then the page CTM (translate 100, 200).
    assert_eq!((x, y), (2. * 10. + 5. + 100., 2. * 20. + 7. + 200.));
}
