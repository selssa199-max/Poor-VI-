//! Turns a title's image into something the Windows loader maps but does
//! not run. Pure byte manipulation, so it is testable on any host.

use victory_pe::layout::{self, dir, dll_flags, file_flags};
use victory_pe::{ImageInfo, PeError};

#[derive(Debug, thiserror::Error)]
pub enum PrepareError {
    #[error(transparent)]
    Pe(#[from] PeError),
    #[error("cannot patch image: {0}")]
    Patch(&'static str),
}

/// A patched copy of an image plus everything we need from the original.
#[derive(Debug, Clone)]
pub struct PreparedImage {
    /// Parsed from the *original* bytes: imports, TLS callbacks and entry
    /// point as the title expects them.
    pub info: ImageInfo,
    /// The patched bytes to hand to the Windows loader.
    pub bytes: Vec<u8>,
    /// BLAKE3 of the original bytes: identifies the exact build.
    pub original_hash: blake3::Hash,
}

impl PreparedImage {
    /// File name for the patched copy, unique per original build.
    pub fn file_name(&self) -> String {
        format!("victory-image-{}.dll", &self.original_hash.to_hex()[..16])
    }
}

/// Produces the patched copy. The changes are:
///
/// * `IMAGE_FILE_DLL` set, entry point cleared: the OS maps it like a DLL
///   and calls nothing;
/// * import directory points to its empty terminator and the bound-import
///   directory is cleared: we bind the original slots using `info.imports`;
/// * TLS callback pointer cleared: the OS still allocates static TLS for
///   the image, but callbacks only run when we call them (after binding);
///   its base relocation is made a no-op so the 0 survives a rebase;
/// * `FORCE_INTEGRITY` cleared, checksum zeroed: the copy is unsigned.
pub fn prepare(original: &[u8]) -> Result<PreparedImage, PrepareError> {
    let info = ImageInfo::parse(original)?;
    let mut bytes = original.to_vec();

    let fh = layout::file_header_offset(&bytes).ok_or(PrepareError::Patch("no file header"))?;
    let opt =
        layout::optional_header_offset(&bytes).ok_or(PrepareError::Patch("no optional header"))?;

    let characteristics = info.characteristics | file_flags::DLL;
    write_u16(
        &mut bytes,
        fh + layout::FILE_HEADER_CHARACTERISTICS,
        characteristics,
    )?;
    write_u32(&mut bytes, opt + layout::OPT_ADDRESS_OF_ENTRY_POINT, 0)?;
    write_u32(&mut bytes, opt + layout::OPT_CHECKSUM, 0)?;
    let dll_characteristics = info.dll_characteristics & !dll_flags::FORCE_INTEGRITY;
    write_u16(
        &mut bytes,
        opt + layout::OPT_DLL_CHARACTERISTICS,
        dll_characteristics,
    )?;

    // Windows 10 assigns static TLS to the prepared DLL when its empty
    // import table is present. The parser has already walked one descriptor
    // per module up to the null terminator, regardless of the recorded Size.
    if let Some(at) = layout::data_directory_offset(&bytes, dir::IMPORT) {
        let rva = layout::read_u32(&bytes, at).ok_or(PrepareError::Patch("no import RVA"))?;
        if rva != 0 {
            let terminator = u32::try_from(info.imports.len())
                .ok()
                .and_then(|count| count.checked_mul(IMPORT_DESCRIPTOR_SIZE))
                .and_then(|offset| rva.checked_add(offset))
                .ok_or(PrepareError::Patch("import terminator RVA overflow"))?;
            write_u32(&mut bytes, at, terminator)?;
            write_u32(&mut bytes, at + 4, IMPORT_DESCRIPTOR_SIZE)?;
        }
    }
    if let Some(at) = layout::data_directory_offset(&bytes, dir::BOUND_IMPORT) {
        write(&mut bytes, at, &[0; layout::DATA_DIRECTORY_SIZE])?;
    }

    if let Some(tls) = &info.tls
        && tls.callbacks_va != 0
    {
        let at = info
            .rva_to_file_offset(tls.directory_rva)
            .ok_or(PrepareError::Patch("TLS directory is not file-backed"))?;
        write_u64(
            &mut bytes,
            at as usize + layout::TLS64_ADDRESS_OF_CALLBACKS,
            0,
        )?;
        // The field's base relocation would turn the 0 into `delta` whenever
        // the loader maps the copy away from its preferred base (Wine always
        // does for DYNAMIC_BASE images), and the loader then reads callbacks
        // from there. Make that entry a no-op.
        let field_rva = tls.directory_rva
            .checked_add(layout::TLS64_ADDRESS_OF_CALLBACKS as u32)
            .ok_or(PrepareError::Patch("TLS callback field RVA overflow"))?;
        for (entry, target) in dir64_relocations(&bytes, &info)? {
            if target == field_rva {
                write_u16(&mut bytes, entry, 0)?; // IMAGE_REL_BASED_ABSOLUTE
            }
        }
    }

    Ok(PreparedImage {
        info,
        bytes,
        original_hash: blake3::hash(original),
    })
}

/// Size of an IMAGE_IMPORT_DESCRIPTOR in the PE file.
const IMPORT_DESCRIPTOR_SIZE: u32 = 20;

/// Every `IMAGE_REL_BASED_DIR64` base relocation: (file offset of its entry,
/// RVA it patches). Reject malformed tables rather than returning partial fixups.
fn dir64_relocations(bytes: &[u8], info: &ImageInfo) -> Result<Vec<(usize, u32)>, PrepareError> {
    let invalid = || PrepareError::Patch("invalid base relocation directory");
    let Some(d) = layout::data_directory_offset(bytes, dir::BASERELOC) else {
        return Ok(Vec::new());
    };
    let rva = layout::read_u32(bytes, d).ok_or_else(invalid)?;
    let size = layout::read_u32(bytes, d + 4).ok_or_else(invalid)?;
    if size == 0 {
        return Ok(Vec::new());
    }
    if rva == 0 || rva.checked_add(size).is_none() {
        return Err(invalid());
    }
    let start = info.rva_to_file_offset(rva).ok_or_else(invalid)? as usize;
    let backed = info.sections.iter().any(|s| {
        let Some(delta) = rva.checked_sub(s.rva) else {
            return false;
        };
        let limit = if s.virtual_size == 0 {
            s.file_size
        } else {
            s.virtual_size.min(s.file_size)
        };
        s.file_offset.checked_add(delta) == Some(start as u32)
            && delta.checked_add(size).is_some_and(|end| end <= limit)
    });
    if !backed {
        return Err(invalid());
    }
    let end = start.checked_add(size as usize).ok_or_else(invalid)?;
    let table = bytes.get(start..end).ok_or_else(invalid)?;
    let mut entries = parse_dir64_blocks(table)?;
    for (offset, _) in &mut entries {
        *offset += start;
    }
    Ok(entries)
}

fn parse_dir64_blocks(mut table: &[u8]) -> Result<Vec<(usize, u32)>, PrepareError> {
    const DIR64: u16 = 10;
    let mut out = Vec::new();
    let mut at = 0;
    while !table.is_empty() {
        let page = layout::read_u32(table, 0)
            .ok_or(PrepareError::Patch("truncated base relocation header"))?;
        let block = layout::read_u32(table, 4)
            .ok_or(PrepareError::Patch("truncated base relocation header"))? as usize;
        if block < 8 || !block.is_multiple_of(2) || block > table.len() {
            return Err(PrepareError::Patch("invalid base relocation block size"));
        }
        for (i, entry) in table[8..block].chunks_exact(2).enumerate() {
            let entry = u16::from_le_bytes([entry[0], entry[1]]);
            if entry >> 12 == DIR64 {
                let target = page.checked_add(u32::from(entry & 0xfff))
                    .ok_or(PrepareError::Patch("base relocation target RVA overflow"))?;
                out.push((at + 8 + i * 2, target));
            }
        }
        at += block;
        table = &table[block..];
    }
    Ok(out)
}

fn write_u16(bytes: &mut [u8], at: usize, v: u16) -> Result<(), PrepareError> {
    write(bytes, at, &v.to_le_bytes())
}
fn write_u32(bytes: &mut [u8], at: usize, v: u32) -> Result<(), PrepareError> {
    write(bytes, at, &v.to_le_bytes())
}
fn write_u64(bytes: &mut [u8], at: usize, v: u64) -> Result<(), PrepareError> {
    write(bytes, at, &v.to_le_bytes())
}
fn write(bytes: &mut [u8], at: usize, v: &[u8]) -> Result<(), PrepareError> {
    bytes
        .get_mut(at..at + v.len())
        .ok_or(PrepareError::Patch("header field outside of file"))?
        .copy_from_slice(v);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn patches_own_test_binary() {
        let original = std::fs::read(std::env::current_exe().unwrap()).unwrap();
        let prepared = prepare(&original).unwrap();
        assert!(!prepared.info.is_dll(), "info describes the original");
        assert!(prepared.info.import_count() > 0);

        let patched = ImageInfo::parse(&prepared.bytes).unwrap();
        assert!(patched.is_dll());
        assert_eq!(patched.entry_rva, 0);
        assert!(patched.imports.is_empty());
        assert_eq!(patched.dll_characteristics & dll_flags::FORCE_INTEGRITY, 0);
        if let Some(tls) = &patched.tls {
            assert_eq!(tls.callbacks_va, 0);
            let field = tls.directory_rva + layout::TLS64_ADDRESS_OF_CALLBACKS as u32;
            assert!(
                dir64_relocations(&prepared.bytes, &patched)
                    .unwrap()
                    .iter()
                    .all(|&(_, target)| target != field),
                "the cleared callback pointer must not be relocated"
            );
        }
        // Nothing else moved: same sections, same size.
        assert_eq!(patched.sections.len(), prepared.info.sections.len());
        assert_eq!(prepared.bytes.len(), original.len());
    }

    #[test]
    fn keeps_iat_slots_when_import_size_excludes_the_terminator() {
        let mut original = std::fs::read(std::env::current_exe().unwrap()).unwrap();
        let info = ImageInfo::parse(&original).unwrap();
        let directory = layout::data_directory_offset(&original, dir::IMPORT).unwrap();
        // This linker layout counts the descriptors for real modules only.
        let descriptor_bytes = u32::try_from(info.imports.len()).unwrap() * IMPORT_DESCRIPTOR_SIZE;
        write_u32(&mut original, directory + 4, descriptor_bytes).unwrap();
        let prepared = prepare(&original).unwrap();
        let empty = layout::read_u32(&prepared.bytes, directory).unwrap();
        assert_eq!(
            empty,
            layout::read_u32(&original, directory).unwrap() + descriptor_bytes
        );
        assert_eq!(
            layout::read_u32(&prepared.bytes, directory + 4),
            Some(IMPORT_DESCRIPTOR_SIZE)
        );
        let at = info.rva_to_file_offset(empty).unwrap() as usize;
        assert_eq!(
            &prepared.bytes[at..at + IMPORT_DESCRIPTOR_SIZE as usize],
            &[0; 20]
        );
        let iat = layout::data_directory_offset(&original, dir::IAT).unwrap();
        assert_eq!(
            &prepared.bytes[iat..iat + layout::DATA_DIRECTORY_SIZE],
            &original[iat..iat + layout::DATA_DIRECTORY_SIZE]
        );
        assert_eq!(prepared.info.import_count(), info.import_count());
        for module in &prepared.info.imports {
            for entry in &module.entries {
                let at = info.rva_to_file_offset(entry.iat_slot_rva).unwrap() as usize;
                assert_eq!(&prepared.bytes[at..at + 8], &original[at..at + 8]);
            }
        }
    }

    #[cfg(windows)]
    #[test]
    fn windows_assigns_static_tls_to_a_prepared_dll() {
        // Model a title DLL using our own linked image. Loading a DLL leaves
        // its assigned index intact; the executable-specific redirect resets it.
        let mut original = std::fs::read(std::env::current_exe().unwrap()).unwrap();
        let flags =
            layout::file_header_offset(&original).unwrap() + layout::FILE_HEADER_CHARACTERISTICS;
        let characteristics =
            ImageInfo::parse(&original).unwrap().characteristics | file_flags::DLL;
        write_u16(&mut original, flags, characteristics).unwrap();
        let prepared = prepare(&original).unwrap();
        let info = &prepared.info;
        let tls = info.tls.as_ref().expect("the test binary uses static TLS");
        let directory = info.rva_to_file_offset(tls.directory_rva).unwrap() as usize;
        let index_va = layout::read_u64(&original, directory + 16).unwrap();
        let index_rva = index_va.checked_sub(info.image_base).unwrap();
        assert!(index_rva + 4 <= u64::from(info.size_of_image));

        let folder = tempfile::tempdir().unwrap();
        let image = crate::load(&prepared, folder.path()).unwrap();
        // SAFETY: the mapped test image contains the checked DWORD above.
        let assigned =
            unsafe { ((image.base() + index_rva as usize) as *const u32).read_unaligned() };
        // SAFETY: the prepared DLL has never been attached or given to a caller.
        unsafe { image.unload() };
        assert_ne!(
            assigned, 0,
            "the process executable already owns TLS slot zero"
        );
    }

    fn relocation_block(page: u32, size: u32, entries: &[u16]) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&page.to_le_bytes());
        bytes.extend_from_slice(&size.to_le_bytes());
        for entry in entries {
            bytes.extend_from_slice(&entry.to_le_bytes());
        }
        bytes
    }

    #[test]
    fn relocation_blocks_keep_dir64_offsets() {
        let mut bytes = relocation_block(0x1000, 12, &[0xa010, 0]);
        bytes.extend(relocation_block(0x2000, 12, &[0xa020, 0]));
        assert_eq!(
            parse_dir64_blocks(&bytes).unwrap(),
            vec![(8, 0x1010), (20, 0x2020)]
        );
        assert!(parse_dir64_blocks(&[]).unwrap().is_empty());
    }

    #[test]
    fn malformed_relocation_blocks_are_rejected() {
        for size in [0, 7, 9, 14, u32::MAX] {
            assert!(parse_dir64_blocks(&relocation_block(0x1000, size, &[0xa010, 0])).is_err());
        }
        let block = relocation_block(0x1000, 12, &[0xa010, 0]);
        for len in 1..block.len() {
            assert!(parse_dir64_blocks(&block[..len]).is_err());
        }
        assert!(parse_dir64_blocks(&relocation_block(u32::MAX, 12, &[0xa001, 0])).is_err());
        let mut bytes = block;
        bytes.extend([0; 4]);
        assert!(parse_dir64_blocks(&bytes).is_err());
    }

    fn relocation_directory_fixture(
        file_size: u32,
        virtual_size: u32,
        directory_offset: u32,
    ) -> (Vec<u8>, ImageInfo, usize) {
        let mut bytes = vec![0u8; 0x200 + file_size as usize];
        bytes[0x3c..0x40].copy_from_slice(&0x40u32.to_le_bytes());
        let opt = 0x58;
        write_u32(&mut bytes, opt + layout::OPT_NUMBER_OF_RVA_AND_SIZES, 16).unwrap();
        let d = layout::data_directory_offset(&bytes, dir::BASERELOC).unwrap();
        write_u32(&mut bytes, d, 0x1000 + directory_offset).unwrap();
        write_u32(&mut bytes, d + 4, 12).unwrap();
        let start = 0x200 + directory_offset as usize;
        bytes[start..start + 12].copy_from_slice(&relocation_block(0x1000, 12, &[0xa008, 0]));
        let info = ImageInfo {
            machine: 0x8664,
            characteristics: 0,
            timestamp: 0,
            subsystem: 0,
            dll_characteristics: 0,
            image_base: 0,
            size_of_image: 0x2000,
            entry_rva: 0,
            stack_reserve: 0,
            stack_commit: 0,
            sections: vec![victory_pe::SectionInfo {
                name: ".reloc".into(),
                rva: 0x1000,
                virtual_size,
                file_offset: 0x200,
                file_size,
                characteristics: 0,
            }],
            imports: Vec::new(),
            delay_imports: Vec::new(),
            exports: Vec::new(),
            tls: None,
            has_relocations: true,
            has_exception_table: false,
            has_load_config: false,
            codeview: None,
        };
        (bytes, info, d)
    }

    #[test]
    fn relocation_directory_may_end_at_the_file_backed_section_boundary() {
        // The final ABSOLUTE entry pads the block to 12 bytes. Exercise both
        // a section with raw file padding and one whose data ends at EOF.
        for (virtual_size, entry_offset) in [(0x1fc, 0x3f8), (0x200, 0x3fc)] {
            let (bytes, info, _) =
                relocation_directory_fixture(0x200, virtual_size, virtual_size - 12);
            assert_eq!(
                dir64_relocations(&bytes, &info).unwrap(),
                vec![(entry_offset, 0x1008)]
            );
        }
    }

    #[test]
    fn relocation_directory_must_be_file_backed() {
        let (mut bytes, info, d) = relocation_directory_fixture(16, 16, 0);
        assert_eq!(dir64_relocations(&bytes, &info).unwrap(), vec![(0x208, 0x1008)]);
        assert!(dir64_relocations(&bytes[..0x20b], &info).is_err());
        write_u32(&mut bytes, d + 4, 20).unwrap();
        assert!(dir64_relocations(&bytes, &info).is_err());
        write_u32(&mut bytes, d, u32::MAX - 3).unwrap();
        assert!(dir64_relocations(&bytes, &info).is_err());
    }

    #[test]
    fn file_name_is_stable_per_build() {
        let original = std::fs::read(std::env::current_exe().unwrap()).unwrap();
        let a = prepare(&original).unwrap();
        let b = prepare(&original).unwrap();
        assert_eq!(a.file_name(), b.file_name());
    }
}
