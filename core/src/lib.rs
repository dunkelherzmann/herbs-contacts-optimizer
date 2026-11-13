//! The shared core of Herb's Contacts Optimizer: the contact model, the file formats and the
//! optimization rules. No window code and no platform code lives here.

pub mod account;
pub mod carddav;
pub mod cards;
pub mod countries;
mod countries_data;
pub mod csv;
pub mod filter;
pub mod ldif;
#[cfg(any(test, feature = "mock"))]
pub mod mock;
pub mod model;
pub mod names;
pub mod optimize;
pub mod text;
pub mod vault;
pub mod vcard;
pub mod xlsx;

pub use model::{Address, Contact, Photo, Remote, Typed};

/// Where the changes to a contact file are saved: a copy beside it named `<name> - optimized.<extension>`,
/// so the original stays as it was. A file that already is such a copy is saved in place.
pub fn optimized_copy(path: &std::path::Path) -> std::path::PathBuf {
    const MARK: &str = " - optimized";
    let stem = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    if stem.ends_with(MARK) {
        return path.to_path_buf();
    }
    let name = match path.extension() {
        Some(ext) => format!("{stem}{MARK}.{}", ext.to_string_lossy()),
        None => format!("{stem}{MARK}"),
    };
    path.with_file_name(name)
}

#[cfg(test)]
mod optimized_copy_tests {
    use super::optimized_copy;
    use std::path::Path;

    #[test]
    fn the_copy_sits_beside_the_original() {
        assert_eq!(optimized_copy(Path::new("/a/Friends.vcf")), Path::new("/a/Friends - optimized.vcf"));
        assert_eq!(optimized_copy(Path::new("/a/my.contacts.csv")), Path::new("/a/my.contacts - optimized.csv"));
        assert_eq!(optimized_copy(Path::new("/a/book")), Path::new("/a/book - optimized"));
        assert_eq!(optimized_copy(Path::new("/a/Friends - optimized.vcf")), Path::new("/a/Friends - optimized.vcf"));
    }
}

/// The file formats the core reads, as far apart as writing them back needs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    /// vCard 2.1 or 3.0; written as 3.0.
    VCard3,
    VCard4,
    CsvGoogle,
    CsvOutlook,
    /// A CSV layout that can be read but not written back column for column.
    CsvOther,
    Ldif,
    JCard,
    XCard,
    Xlsx,
}

impl Format {
    /// Every format that can be written, in the order the Export menu lists them.
    pub const WRITABLE: [Format; 8] = [Format::VCard3, Format::VCard4, Format::CsvGoogle, Format::CsvOutlook, Format::Xlsx, Format::Ldif, Format::JCard, Format::XCard];

    /// The name for menus.
    pub fn label(self) -> &'static str {
        match self {
            Format::VCard3 => "vCard 3.0",
            Format::VCard4 => "vCard 4.0",
            Format::CsvGoogle => "CSV for Google",
            Format::CsvOutlook => "CSV for Outlook",
            Format::CsvOther => "CSV",
            Format::Ldif => "LDIF (Thunderbird)",
            Format::JCard => "jCard (JSON)",
            Format::XCard => "xCard (XML)",
            Format::Xlsx => "Excel Workbook",
        }
    }

    /// The ending a file of this format has.
    pub fn extension(self) -> &'static str {
        match self {
            Format::VCard3 | Format::VCard4 => "vcf",
            Format::CsvGoogle | Format::CsvOutlook | Format::CsvOther => "csv",
            Format::Ldif => "ldif",
            Format::JCard => "json",
            Format::XCard => "xml",
            Format::Xlsx => "xlsx",
        }
    }

    /// The contacts in this format, or `None` when it can only be read.
    pub fn write(self, contacts: &[Contact]) -> Option<Vec<u8>> {
        let text = match self {
            Format::VCard3 => vcard::write(contacts, vcard::Version::V3),
            Format::VCard4 => vcard::write(contacts, vcard::Version::V4),
            Format::CsvGoogle => csv::write_google(contacts),
            Format::CsvOutlook => csv::write_outlook(contacts),
            Format::CsvOther => return None,
            Format::Ldif => ldif::write(contacts),
            Format::JCard => cards::write_jcard(contacts),
            Format::XCard => cards::write_xcard(contacts),
            Format::Xlsx => return xlsx::write(contacts).ok(),
        };
        Some(text.into_bytes())
    }
}

/// Reads contacts from a file's bytes. The format is recognised from the content, not the name.
pub fn read(bytes: &[u8]) -> Result<(Format, Vec<Contact>), String> {
    if xlsx::is_xlsx(bytes) {
        return Ok((Format::Xlsx, xlsx::parse(bytes)?));
    }
    let text = text::decode(bytes);
    let upper = text.to_ascii_uppercase();
    if cards::is_jcard(&text) {
        Ok((Format::JCard, cards::parse_jcard(&text)?))
    } else if cards::is_xcard(&text) {
        Ok((Format::XCard, cards::parse_xcard(&text)?))
    } else if upper.contains("BEGIN:VCARD") {
        let format = if upper.contains("VERSION:4.0") { Format::VCard4 } else { Format::VCard3 };
        Ok((format, vcard::parse(&text)))
    } else if ldif::is_ldif(&text) {
        Ok((Format::Ldif, ldif::parse(&text)))
    } else {
        let contacts = csv::parse(&text)?;
        let header = text.lines().next().unwrap_or_default();
        let google = header.contains(" - Value") || header.contains("Given Name");
        let format = if google {
            Format::CsvGoogle
        } else if csv::is_outlook(header) {
            Format::CsvOutlook
        } else {
            Format::CsvOther
        };
        Ok((format, contacts))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_format_is_recognised_and_written_back() {
        let (format, contacts) = read(b"BEGIN:VCARD\nVERSION:4.0\nFN:Ana\nEND:VCARD\n").unwrap();
        assert_eq!(format, Format::VCard4);
        assert!(String::from_utf8(format.write(&contacts).unwrap()).unwrap().contains("VERSION:4.0"));
        assert_eq!(read(b"BEGIN:VCARD\nVERSION:2.1\nFN:Ana\nEND:VCARD\n").unwrap().0, Format::VCard3);
        assert_eq!(read(b"Name,Phone 1 - Value\nAna,555 0100\n").unwrap().0, Format::CsvGoogle);
        assert_eq!(read(b"First Name,Business Phone\nAna,555 0100\n").unwrap().0, Format::CsvOutlook);
        let (format, contacts) = read(b"First Name,Mobile Number\nAna,555 0100\n").unwrap();
        assert_eq!(format, Format::CsvOther);
        assert!(format.write(&contacts).is_none());

        // Every format that can be written is recognised again from what it wrote.
        let ana = vec![Contact { display_name: "Ana".into(), first: "Ana".into(), ..Contact::default() }];
        for format in Format::WRITABLE {
            let (found, back) = read(&format.write(&ana).unwrap()).unwrap();
            assert_eq!(found, format, "{}", format.label());
            assert_eq!(back.len(), 1, "{}", format.label());
            assert_eq!(back[0].first, "Ana", "{}", format.label());
        }
    }
}
