use std::{cell::RefCell, collections::HashMap, env, fmt, rc::Rc};

use lopdf::Document;
use pdf_extract::{ConvertToFmt, OutputDev, OutputError, PlainTextOutput};

fn main() {
    let file = env::args().nth(1).unwrap();
    let doc = Document::load(file).unwrap();
    let mut output = PagePlainTextOutput::new();
    pdf_extract::output_doc(&doc, &mut output).unwrap();

    // print the text of each page
    for (page_num, text) in output.pages {
        println!("Page {}: {}", page_num, text);
    }
}

struct PagePlainTextOutput {
    inner: PlainTextOutput<OutputWrapper>,
    pages: HashMap<u32, String>,
    current_page: u32,
    reader: Rc<RefCell<String>>,
}

struct OutputWrapper(Rc<RefCell<String>>);

impl std::fmt::Write for OutputWrapper {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        self.0.borrow_mut().write_str(s).map_err(|_| fmt::Error)
    }
}

impl ConvertToFmt for OutputWrapper {
    type Writer = OutputWrapper;

    fn convert(self) -> Self::Writer {
        self
    }
}

impl PagePlainTextOutput {
    fn new() -> Self {
        let s = Rc::new(RefCell::new(String::new()));
        let writer = Rc::clone(&s);
        Self {
            pages: HashMap::new(),
            current_page: 0,
            reader: s,
            inner: PlainTextOutput::new(OutputWrapper(writer)),
        }
    }
}

impl OutputDev for PagePlainTextOutput {
    fn begin_page(
        &mut self,
        page_num: u32,
        media_box: &pdf_extract::MediaBox,
        art_box: Option<(f64, f64, f64, f64)>,
    ) -> Result<(), OutputError> {
        self.current_page = page_num;
        self.inner.begin_page(page_num, media_box, art_box)
    }

    fn end_page(&mut self) -> Result<(), OutputError> {
        self.inner.end_page()?;

        let buf = self.reader.borrow().clone();
        self.pages.insert(self.current_page, buf);
        self.reader.borrow_mut().clear();

        Ok(())
    }

    fn output_character(
        &mut self,
        trm: &pdf_extract::Transform,
        width: f64,
        spacing: f64,
        font_size: f64,
        char: &str,
    ) -> Result<(), OutputError> {
        self.inner
            .output_character(trm, width, spacing, font_size, char)
    }

    fn begin_word(&mut self) -> Result<(), OutputError> {
        self.inner.begin_word()
    }

    fn end_word(&mut self) -> Result<(), OutputError> {
        self.inner.end_word()
    }

    fn end_line(&mut self) -> Result<(), OutputError> {
        self.inner.end_line()
    }
}
