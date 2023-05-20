//! Firmware file formats
use std::str;
use std::{borrow::Cow, path::Path};

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

pub fn read_firmware_from_file<P: AsRef<Path>>(path: P) -> Result<Vec<u8>> {
    let p = path.as_ref();
    let raw = std::fs::read(p)?;

    let format = guess_format(p, &raw);
    log::info!("Read {} as {:?} format", p.display(), format);
    match format {
        FirmwareFormat::PlainHex => Ok(hex::decode(
            raw.into_iter()
                .filter(|&c| c != b'\r' || c != b'\n')
                .collect::<Vec<u8>>(),
        )?),
        FirmwareFormat::IntelHex => Ok(read_ihex(str::from_utf8(&raw)?)?),
        FirmwareFormat::ELF => Ok(objcopy_binary(&raw)?),
        FirmwareFormat::Binary => Ok(raw),
    }
}

pub fn guess_format(path: &Path, raw: &[u8]) -> FirmwareFormat {
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

pub fn read_ihex(data: &str) -> Result<Vec<u8>> {
    use ihex::Record;

    let mut base_address = 0;

    let mut records = vec![];
    for record in ihex::Reader::new(&data) {
        let record = record?;
        use Record::*;
        match record {
            Data { offset, value } => {
                let offset = base_address + offset as u32;

                records.push((offset, value.into()));
            }
            EndOfFile => (),
            ExtendedSegmentAddress(address) => {
                base_address = (address as u32) * 16;
            }
            StartSegmentAddress { .. } => (),
            ExtendedLinearAddress(address) => {
                base_address = (address as u32) << 16;
            }
            StartLinearAddress(_) => (),
        };
    }
    merge_sections(records)
}

/// Simulates `objcopy -O binary`.
pub fn objcopy_binary(elf_data: &[u8]) -> Result<Vec<u8>> {
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
            log::info!(
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
            sections.push((p_paddr as u32, section_data.into()));
            log::debug!("Section names: {:?}", section_names);
        }
    }

    if sections.is_empty() {
        anyhow::bail!("empty ELF file");
    }
    log::debug!("found {} sections", sections.len());
    merge_sections(sections)
}

fn merge_sections(mut sections: Vec<(u32, Cow<[u8]>)>) -> Result<Vec<u8>> {
    sections.sort(); // order by start address

    let start_address = sections.first().unwrap().0;
    let end_address = sections.last().unwrap().0 + sections.last().unwrap().1.len() as u32;

    let total_size = end_address - start_address;

    let mut binary = vec![0u8; total_size as usize];
    // FIXMME: check section overlap?
    for (addr, sect) in sections {
        let sect_start = (addr - start_address) as usize;
        let sect_end = (addr - start_address) as usize + sect.len();
        binary[sect_start..sect_end].copy_from_slice(&sect);
    }
    Ok(binary)
}
