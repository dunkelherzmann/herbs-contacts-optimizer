//! Excel workbooks: the first sheet is read as a table of contacts with a header row, and
//! written in Google's column layout.

use crate::csv;
use crate::model::Contact;
use calamine::Reader;

/// True for the bytes of a workbook (a zip archive, as `.xlsx` files are).
pub fn is_xlsx(bytes: &[u8]) -> bool {
    bytes.starts_with(b"PK\x03\x04")
}

pub fn parse(bytes: &[u8]) -> Result<Vec<Contact>, String> {
    let unreadable = |e: &dyn std::fmt::Display| format!("This workbook could not be read: {e}");
    let mut workbook = calamine::open_workbook_auto_from_rs(std::io::Cursor::new(bytes)).map_err(|e| unreadable(&e))?;
    let sheet = workbook.worksheet_range_at(0).ok_or("The workbook has no sheet.")?.map_err(|e| unreadable(&e))?;
    let rows: Vec<Vec<String>> = sheet.rows().map(|row| row.iter().map(|cell| cell.to_string()).collect()).filter(|row: &Vec<String>| row.iter().any(|cell| !cell.is_empty())).collect();
    csv::from_rows(rows)
}

pub fn write(contacts: &[Contact]) -> Result<Vec<u8>, String> {
    let mut workbook = rust_xlsxwriter::Workbook::new();
    let sheet = workbook.add_worksheet();
    sheet.set_name("Contacts").map_err(|e| e.to_string())?;
    for (row, cells) in csv::google_rows(contacts).iter().enumerate() {
        for (column, cell) in cells.iter().enumerate() {
            // Everything goes in as text, so a phone number keeps its plus and its leading zeros.
            sheet.write_string(row as u32, column as u16, cell).map_err(|e| e.to_string())?;
        }
    }
    workbook.save_to_buffer().map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Typed;

    #[test]
    fn a_workbook_written_reads_back_the_same() {
        let mut ana = Contact { first: "Ana".into(), last: "García".into(), display_name: "Ana García".into(), note: "a,b\nc".into(), company: "Acme".into(), ..Contact::default() };
        ana.phones = vec![Typed::new("mobile", "+34 600 111 222"), Typed::new("work", "0170 1234567")];
        ana.emails = vec![Typed::new("home", "ana@example.org")];
        ana.groups = vec!["Friends".into()];
        let all = vec![ana, Contact { display_name: "Bob".into(), ..Contact::default() }];
        let bytes = write(&all).unwrap();
        assert!(is_xlsx(&bytes));
        assert_eq!(parse(&bytes).unwrap(), all);
        assert!(parse(b"PK\x03\x04 not really a workbook").is_err());
    }
}
