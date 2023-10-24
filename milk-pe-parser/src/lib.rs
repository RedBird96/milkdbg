pub mod headers;
mod helpers;

use auto_from::From;
use headers::*;
use std::{path::Path, str::Utf8Error};

#[derive(Debug, From)]
#[auto_from]
pub enum ParseError {
    IO(std::io::Error),
    UTF8(Utf8Error),
    OutOfBounds,
    WrongSignature,
    Unknown,
}

impl From<nom::Err<nom::error::Error<&[u8]>>> for ParseError {
    fn from(_: nom::Err<nom::error::Error<&[u8]>>) -> Self {
        ParseError::Unknown
    }
}

pub enum PeOptional {
    PE32(PeOptionalHeader32),
}

impl PeOptional {
    pub fn get_image_base(&self) -> usize {
        match self {
            PeOptional::PE32(pe) => pe.image_base as usize,
        }
    }

    pub fn get_code_base(&self) -> RVA {
        match self {
            PeOptional::PE32(pe) => pe.base_of_code,
        }
    }

    pub fn get_code(&self) -> (RVA, usize) {
        match self {
            PeOptional::PE32(pe) => (pe.base_of_code, pe.size_of_code as usize),
        }
    }

    pub fn get_address_of_entry_point(&self) -> RVA {
        match self {
            PeOptional::PE32(pe) => pe.address_of_entry_point,
        }
    }
}

#[derive(Debug)]
pub struct RawImportThunkData {
    pub data: u32,
}

#[derive(Debug)]
pub enum ThunkData {
    Ordinal(u32),
    ImportedByName { hint: u16, name: String },
}

pub struct PE {
    pub dos_header: PeDosHeader,
    pub coff_header: PeCoffHeader,
    pub optional: PeOptional,
    pub data_directory: Vec<PeDataDirectory>,
    bytes: Vec<u8>,
}

impl PE {
    pub fn parse<P: AsRef<Path>>(path: P) -> Result<PE, ParseError> {
        let path = path.as_ref();
        let bytes = std::fs::read(path)?;
        Self::from_vec(bytes)
    }

    pub fn from_vec(bytes: Vec<u8>) -> Result<PE, ParseError> {
        let s = bytes.as_slice();

        // DOS Header
        let (_, dos_header) = helpers::parse::<PeDosHeader>(s)?;

        // Signature
        let (s, _) = helpers::take(s, dos_header.e_lfanew.into())?;
        let (s, signature) = helpers::parse::<u32>(s)?;
        if *signature != 0x00004550 {
            return Err(ParseError::WrongSignature);
        }

        // COFF File Header
        let (s, coff_header) = helpers::parse::<PeCoffHeader>(s)?;

        // Optional Header (Image Only)
        let (s, optional, qty_data) = match coff_header.machine {
            ImageFileMachine::I386 => {
                let (s, opt32_header) = helpers::parse::<PeOptionalHeader32>(s)?;

                (
                    s,
                    PeOptional::PE32(opt32_header.clone()),
                    opt32_header.number_of_rva_and_sizes as usize,
                )
            }
            _ => todo!(),
        };

        // Data dictionary
        let (_, data_directory) = helpers::parse_slice::<PeDataDirectory>(s, qty_data)?;

        Ok(PE {
            dos_header: dos_header.clone(),
            coff_header: coff_header.clone(),
            optional,
            data_directory: data_directory.iter().map(|x| x.clone()).collect(),
            bytes,
        })
    }

    pub fn get_data_directory(&self, i: usize) -> Option<&PeDataDirectory> {
        self.data_directory.get(i)
    }

    pub fn get_export_section(&self) -> Option<&PeExportSection> {
        let s = self.bytes.as_slice();
        let data = &self.data_directory[0];
        if data.virtual_address.0 == 0 {
            None
        } else {
            let (s, _) = helpers::take(s, data.virtual_address.0 as usize).ok()?;
            let (_, section) = helpers::parse::<PeExportSection>(s).ok()?;
            Some(section)
        }
    }

    pub fn get_imports(&self) -> Vec<&PeImportBinary> {
        let mut executables = vec![];

        let s = self.bytes.as_slice();
        let data = &self.data_directory[1];
        if data.virtual_address.0 != 0 {
            let mut offset = data.virtual_address.0 as usize;
            loop {
                let (_, section) = helpers::parse::<PeImportBinary>(&s[offset..]).unwrap();
                if section.original_first_thunk.0 == 0 {
                    break;
                }

                offset += std::mem::size_of::<PeImportBinary>();

                executables.push(section);
            }
        }
        executables
    }

    pub fn get_iat(&self) -> Vec<&PeImportBinary> {
        let mut executables = vec![];

        let s = self.bytes.as_slice();
        let data = &self.data_directory[12];
        println!(
            "{} {:X?}",
            self.data_directory.len(),
            data.virtual_address.0 as usize
        );
        if data.virtual_address.0 != 0 {
            let mut offset = data.virtual_address.0 as usize;
            loop {
                let (_, bin) = helpers::parse::<PeImportBinary>(&s[offset..]).unwrap();
                println!("{:?}", bin);
                if bin.original_first_thunk.0 == 0 {
                    break;
                }

                offset += std::mem::size_of::<PeImportBinary>();

                executables.push(bin);
            }
        }
        executables
    }

    pub fn get_raw_import_thunks_of(
        &self,
        import: &PeImportBinary,
        original: bool,
    ) -> Vec<&RawImportThunkData> {
        let mut thunks = vec![];

        let s = self.bytes.as_slice();
        let mut offset = if original {
            import.original_first_thunk.0 as usize
        } else {
            import.first_thunk.0 as usize
        };

        loop {
            let (_, t) = helpers::parse::<RawImportThunkData>(&s[offset..]).unwrap();
            if t.data == 0 {
                break;
            }

            offset += std::mem::size_of::<RawImportThunkData>();

            thunks.push(t);
        }

        thunks
    }

    pub fn get_thunk_from_raw(&self, thunk: &RawImportThunkData) -> Result<ThunkData, ParseError> {
        if thunk.data & 0x80000000 > 0 {
            Ok(ThunkData::Ordinal(thunk.data & 0x7FFFFFFF))
        } else {
            let s = self.bytes.as_slice();
            let offset = thunk.data as usize;
            if offset > s.len() {
                Err(ParseError::OutOfBounds)
            } else {
                let (s, &hint) = helpers::parse::<u16>(&s[offset..]).unwrap();
                let (_, name) = helpers::take_untill_value(s, 0).unwrap();
                let name = std::str::from_utf8(name).unwrap().to_string();
                Ok(ThunkData::ImportedByName { hint, name })
            }
        }
    }

    pub fn read_at<T>(&self, rva: RVA) -> Result<&T, ParseError> {
        let s = self.bytes.as_slice();
        let offset = rva.0 as usize;
        if offset > s.len() {
            Err(ParseError::OutOfBounds)
        } else {
            let (_, v) = helpers::parse::<T>(&s[offset..])?;
            Ok(v)
        }
    }

    pub fn read_null_terminated_string_at(&self, rva: RVA) -> Result<&str, ParseError> {
        let s = self.bytes.as_slice();
        let offset = rva.0 as usize;
        let (_, str) = helpers::take_untill_value(&s[offset..], 0)?;
        let str = std::str::from_utf8(str)?;
        Ok(str)
    }

    pub fn read_possible_null_terminated_string_at(&self, rva: RVA) -> Option<String> {
        let s = self.bytes.as_slice();
        let mut offset = rva.0 as usize;

        let mut string = String::new();
        loop {
            if offset > s.len() {
                return None;
            }
            let c = s[offset] as char;

            if c == '\0' {
                break;
            }

            if !c.is_ascii_alphanumeric() && !c.is_ascii_punctuation() && !c.is_ascii_whitespace() {
                return None;
            }

            string.push(c);
            offset += 1;
        }

        Some(string)
    }

    pub fn get_code(&self) -> &[u8] {
        let (base, size) = self.optional.get_code();
        let start = base.0 as usize;
        let end = start + size as usize;
        let s = self.bytes.as_slice();
        &s[start..end]
    }
}
