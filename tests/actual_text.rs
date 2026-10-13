use lopdf::{dictionary, Dictionary, Document, Object, Stream};

fn extract(content: &str, properties: Option<Dictionary>) -> String {
    extract_with_form(content, properties, None)
}

fn extract_with_form(content: &str, properties: Option<Dictionary>, form: Option<&str>) -> String {
    extract_pages(&[content], properties, form, "1.7")
}

fn extract_pages(
    contents: &[&str],
    properties: Option<Dictionary>,
    form: Option<&str>,
    version: &str,
) -> String {
    let mut document = Document::with_version(version);
    let pages_id = document.new_object_id();
    let font_id = document.add_object(dictionary! {
        "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica",
        "Encoding" => "WinAnsiEncoding",
    });
    let mut resources = dictionary! { "Font" => dictionary! { "F1" => font_id } };
    if let Some(properties) = properties {
        let properties_id = document.add_object(properties);
        resources.set("Properties", properties_id);
    }
    if let Some(form) = form {
        let form_id = document.add_object(Stream::new(
            dictionary! {
                "Type" => "XObject", "Subtype" => "Form",
                "BBox" => vec![0.into(), 0.into(), 300.into(), 100.into()],
                "Resources" => resources.clone(),
            },
            form.as_bytes().to_vec(),
        ));
        resources.set("XObject", dictionary! { "Drawing" => form_id });
    }
    let mut pages = Vec::new();
    for content in contents {
        let content_id =
            document.add_object(Stream::new(Dictionary::new(), content.as_bytes().to_vec()));
        let page_id = document.add_object(dictionary! {
            "Type" => "Page", "Parent" => pages_id, "Resources" => resources.clone(),
            "MediaBox" => vec![0.into(), 0.into(), 300.into(), 100.into()],
            "Contents" => content_id,
        });
        pages.push(Object::Reference(page_id));
    }
    document.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages", "Kids" => pages, "Count" => contents.len() as i64,
        }),
    );
    let catalog_id = document.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
    document.trailer.set("Root", catalog_id);
    let mut bytes = Vec::new();
    document.save_to(&mut bytes).unwrap();
    pdf_extract::extract_text_from_mem(&bytes).unwrap()
}

#[test]
fn actual_text_replaces_positioned_glyphs_once_and_keeps_following_text() {
    let text = extract(
        "BT /F1 18 Tf 10 50 Td /Span << /ActualText (Beta) >> BDC
         [(Be) -300 (t) -300 (a)] TJ EMC ( suffix) Tj ET",
        None,
    );
    assert_eq!(text.trim(), "Beta suffix");
}

#[test]
fn actual_text_decodes_unicode_and_supplementary_characters() {
    let value = "Καλημέρα 🌐";
    let encoded = std::iter::once(0xfeff)
        .chain(value.encode_utf16())
        .map(|unit| format!("{unit:04X}"))
        .collect::<String>();
    let text = extract(
        &format!(
            "BT /F1 18 Tf 10 50 Td /Span << /ActualText <{encoded}> >> BDC (glyphs) Tj EMC ET"
        ),
        None,
    );
    assert_eq!(text.trim(), value);
}

#[test]
fn actual_text_resolves_named_property_lists() {
    let properties =
        dictionary! { "Label" => dictionary! { "ActualText" => Object::string_literal("office") } };
    let text = extract(
        "BT /F1 18 Tf 10 50 Td /Span /Label BDC (o f f i c e) Tj EMC ET",
        Some(properties),
    );
    assert_eq!(text.trim(), "office");
}

#[test]
fn outer_actual_text_replaces_nested_content() {
    let text = extract(
        "BT /F1 18 Tf 10 50 Td /Span << /ActualText (outer) >> BDC
         (A) Tj /Span << /ActualText (inner) >> BDC (B) Tj EMC (C) Tj EMC ET",
        None,
    );
    assert_eq!(text.trim(), "outer");
}

#[test]
fn empty_actual_text_suppresses_its_glyphs() {
    let text = extract(
        "BT /F1 18 Tf 10 50 Td (before) Tj
         /Span << /ActualText () >> BDC (ignored) Tj EMC (after) Tj ET",
        None,
    );
    assert_eq!(text.trim(), "beforeafter");
}

#[test]
fn actual_text_also_replaces_graphic_content() {
    let text = extract(
        "BT /F1 18 Tf 10 50 Td (before ) Tj ET
         /Span << /ActualText ([star]) >> BDC 100 50 10 10 re f EMC
         BT /F1 18 Tf 160 50 Td ( after) Tj ET",
        None,
    );
    assert_eq!(
        text.split_whitespace().collect::<Vec<_>>(),
        ["before", "[star]", "after"]
    );
}

#[test]
fn outer_actual_text_also_covers_form_content() {
    let text = extract_with_form(
        "/Span << /ActualText (outer) >> BDC /Drawing Do EMC",
        None,
        Some("BT /F1 18 Tf 10 50 Td /Span << /ActualText (inner) >> BDC (glyphs) Tj EMC ET"),
    );
    assert_eq!(text.trim(), "outer");
}

#[test]
fn malformed_or_missing_properties_preserve_ordinary_text() {
    for property in [
        "/Missing",
        "<< /ActualText 42 >>",
        "<< /ActualText /Name >>",
    ] {
        let text = extract(
            &format!("BT /F1 18 Tf 10 50 Td /Span {property} BDC (visible) Tj EMC ET"),
            None,
        );
        assert_eq!(text.trim(), "visible");
    }
}

#[test]
fn empty_utf16_replacement_and_invalid_surrogate_do_not_panic() {
    for (value, expected) in [("FEFF", ""), ("FEFF0041D800", "A\u{fffd}")] {
        let text = extract(
            &format!(
                "BT /F1 18 Tf 10 50 Td /Span << /ActualText <{value}> >> BDC (glyphs) Tj EMC ET"
            ),
            None,
        );
        assert_eq!(text.trim(), expected);
    }
}

#[test]
fn pdf2_actual_text_decodes_utf8_strings() {
    let value = "日本語 🌐";
    let encoded = [0xef, 0xbb, 0xbf]
        .iter()
        .chain(value.as_bytes())
        .map(|byte| format!("{byte:02X}"))
        .collect::<String>();
    let content =
        format!("BT /F1 18 Tf 10 50 Td /Span << /ActualText <{encoded}> >> BDC (glyphs) Tj EMC ET");
    let text = extract_pages(&[&content], None, None, "2.0");
    assert_eq!(text.trim(), value);
}

#[test]
fn unclosed_replacement_does_not_suppress_the_next_page() {
    let following_page = "BT /F1 18 Tf 10 50 Td (second) Tj ET";
    let text = extract_pages(
        &[
            "BT /F1 18 Tf 10 50 Td /Span << /ActualText (first) >> BDC (glyphs) Tj ET",
            following_page,
        ],
        None,
        None,
        "1.7",
    );
    let closed = extract_pages(
        &[
            "BT /F1 18 Tf 10 50 Td /Span << /ActualText (first) >> BDC (glyphs) Tj EMC ET",
            following_page,
        ],
        None,
        None,
        "1.7",
    );
    assert_eq!(text, closed);
    assert!(text.contains("second"), "{text:?}");
}
