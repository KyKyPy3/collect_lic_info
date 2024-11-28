use std::collections::HashMap;
use xlsxwriter::format::{FormatAlignment, FormatColor, FormatUnderline};
use xlsxwriter::Format;

pub struct WorkbookFormatter {
  formats: HashMap<String, Format>,
}

impl WorkbookFormatter {
  pub fn new() -> Self {
    let mut formats = HashMap::new();
    let mut url_format = Format::new();

    url_format
      .set_font_color(FormatColor::Blue)
      .set_underline(FormatUnderline::Single)
      .set_align(FormatAlignment::Left);

    formats.insert("url".to_owned(), url_format);

    Self { formats }
  }

  pub fn url_format(&self) -> Option<&Format> {
    self.formats.get("url")
  }
}
