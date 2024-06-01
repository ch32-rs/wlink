//! Firmware file formats
use std::path::Path;
use std::str;

use anyhow::Result;
use object::{
    elf::FileHeader32, elf::PT_LOAD, read::elf::FileHeader, read::elf::ProgramHeader, Endianness,
    Object, ObjectSection,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FirmwareFormat {
    PlainHex,
    IntelHex,
    ELF,
    Binary,
}

#[derive(Debug, Clone)]
pub struct Section {
    /// The start address of the segment, physical address.
    pub address: u32,
    pub data: Vec<u8>,
}

impl Section {
    pub fn end_address(&self) -> u32 {
        self.address + self.data.len() as u32
    }
}

/// The abstract representation of a firmware image.
#[derive(Debug, Clone)]
pub enum Firmware {
    /// A single section, with address undefined.
    Binary(Vec<u8>),
    /// Multiple sections, with different addresses.
    Sections(Vec<Section>),
}

impl Firmware {
    /// Merge sections, and fill gap with 0xff
    pub fn merge_sections(self) -> Result<Self> {
        let Firmware::Sections(mut sections) = self else {
            return Ok(self);
        };
        sections.sort_by_key(|s| s.address);
        let mut merged = vec![];

        let mut it = sections.drain(0..);
        let mut last = it
            .next()
            .expect("firmware must has at least one section; qed");

        for sect in it {
            if let Some(gap) = sect.address.checked_sub(last.end_address()) {
                if gap > 0 {
                    log::debug!("Merge firmware sections with gap: {}", gap);
                }
                last.data.resize(last.data.len() + gap as usize, 0xff); // fill gap with 0xff
                last.data.extend_from_slice(&sect.data);
            } else {
                return Err(anyhow::format_err!(
                    "section address overflow: {:#010x} + {:#x}",
                    last.address,
                    last.data.len()
                ));
            }
        }
        merged.push(last);
        Ok(Firmware::Sections(merged))
    }
}

pub fn read_firmware_from_file<P: AsRef<Path>>(path: P) -> Result<Firmware> {
    let p = path.as_ref();
    let raw = std::fs::read(p)?;

    let format = guess_format(p, &raw);
    log::info!("Read {} as {:?} format", p.display(), format);
    match format {
        FirmwareFormat::PlainHex => {
            let raw = hex::decode(
                raw.into_iter()
                    .filter(|&c| c != b'\r' || c != b'\n')
                    .collect::<Vec<u8>>(),
            )?;
            Ok(Firmware::Binary(raw))
        }
        FirmwareFormat::Binary => Ok(Firmware::Binary(raw)),
        FirmwareFormat::IntelHex => {
            read_ihex(str::from_utf8(&raw)?).and_then(|f| f.merge_sections())
        }
        FirmwareFormat::ELF => read_elf(&raw).and_then(|f| f.merge_sections()),
    }
}

fn guess_format(path: &Path, raw: &[u8]) -> FirmwareFormat {
    let ext = path
        .extension()
        .map(|s| s.to_string_lossy())
        .unwrap_or_default()
        .to_lowercase();
    if ["ihex", "ihe", "h86", "hex", "a43", "a90"].contains(&&*ext) {
        return FirmwareFormat::IntelHex;
    }

    // FIXME: is this 4-byte possible to be some kind of assembly binary?
    if raw.starts_with(&[0x7f, b'E', b'L', b'F']) {
        FirmwareFormat::ELF
    } else if raw[0] == b':'
        && raw
            .iter()
            .all(|&c| (c as char).is_ascii_hexdigit() || c == b':' || c == b'\n' || c == b'\r')
    {
        FirmwareFormat::IntelHex
    } else if raw
        .iter()
        .all(|&c| (c as char).is_ascii_hexdigit() || c == b'\n' || c == b'\r')
    {
        FirmwareFormat::PlainHex
    } else {
        FirmwareFormat::Binary
    }
}

pub fn read_hex(data: &str) -> Result<Vec<u8>> {
    Ok(hex::decode(data)?)
}

pub fn read_ihex(data: &str) -> Result<Firmware> {
    use ihex::Record::*;

    let mut base_address = 0;

    let mut segs: Vec<Section> = vec![];
    let mut last_end_address = 0;
    for record in ihex::Reader::new(data) {
        let record = record?;
        match record {
            Data { offset, value } => {
                let start_address = base_address + offset as u32;

                if let Some(last) = segs.last_mut() {
                    if start_address == last_end_address {
                        // merge to last
                        last_end_address = start_address + value.len() as u32;
                        last.data.extend_from_slice(&value);

                        continue;
                    }
                }

                last_end_address = start_address + value.len() as u32;
                segs.push(Section {
                    address: start_address,
                    data: value.to_vec(),
                })
            }
            ExtendedSegmentAddress(address) => {
                base_address = (address as u32) * 16;
            }
            ExtendedLinearAddress(address) => {
                base_address = (address as u32) << 16;
            }
            StartSegmentAddress { .. } => (),
            StartLinearAddress(_) => (),
            EndOfFile => (),
        };
    }

    Ok(Firmware::Sections(segs))
}

/// Simulates `objcopy -O binary`, returns loadable sections
pub fn read_elf(elf_data: &[u8]) -> Result<Firmware> {
    let file_kind = object::FileKind::parse(elf_data)?;

    match file_kind {
        object::FileKind::Elf32 => (),
        _ => anyhow::bail!("cannot read file as ELF32 format"),
    }
    let elf_header = FileHeader32::<Endianness>::parse(elf_data)?;
    let binary = object::read::elf::ElfFile::<FileHeader32<Endianness>>::parse(elf_data)?;

    let mut sections = vec![];

    let endian = elf_header.endian()?;

    // Ref: https://docs.oracle.com/cd/E19683-01/816-1386/chapter6-83432/index.html
    for segment in elf_header.program_headers(elf_header.endian()?, elf_data)? {
        // Get the physical address of the segment. The data will be programmed to that location.
        let p_paddr: u64 = segment.p_paddr(endian).into();
        // Virtual address
        let p_vaddr: u64 = segment.p_vaddr(endian).into();

        let flags = segment.p_flags(endian);

        let segment_data = segment
            .data(endian, elf_data)
            .map_err(|_| anyhow::format_err!("Failed to access data for an ELF segment."))?;
        if !segment_data.is_empty() && segment.p_type(endian) == PT_LOAD {
            log::debug!(
                    "Found loadable segment, physical address: {:#010x}, virtual address: {:#010x}, flags: {:#x}",
                    p_paddr,
                    p_vaddr,
                    flags
                );
            let (segment_offset, segment_filesize) = segment.file_range(endian);
            let mut section_names = vec![];
            for section in binary.sections() {
                let (section_offset, section_filesize) = match section.file_range() {
                    Some(range) => range,
                    None => continue,
                };
                if section_filesize == 0 {
                    continue;
                }

                // contains range
                if segment_offset <= section_offset
                    && segment_offset + segment_filesize >= section_offset + section_filesize
                {
                    log::debug!(
                        "Matching section: {:?} offset: 0x{:x} size: 0x{:x}",
                        section.name()?,
                        section_offset,
                        section_filesize
                    );
                    for (offset, relocation) in section.relocations() {
                        log::debug!("Relocation: offset={}, relocation={:?}", offset, relocation);
                    }
                    section_names.push(section.name()?.to_owned());
                }
            }
            let section_data = &elf_data[segment_offset as usize..][..segment_filesize as usize];
            sections.push(Section {
                address: p_paddr as u32,
                data: section_data.to_vec(),
            });
            log::debug!("Section names: {:?}", section_names);
        }
    }

    if sections.is_empty() {
        anyhow::bail!("empty ELF file");
    }
    log::debug!("found {} sections", sections.len());
    // merge_sections(sections)
    Ok(Firmware::Sections(sections))
}
