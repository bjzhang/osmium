use core::slice;
use paging;

const ELF_MAGIC: u32 = 0x464c457f;

pub struct Elf<'a> {
    bytes: &'a [u8],
    pub elf: &'a ElfHeader,
}

#[derive(Debug)]
pub enum ElfError {
    InvalidMagic,
}

impl<'a> Elf<'a> {
    pub fn new(bytes: *const [u8]) -> Result<Elf<'a>, ElfError> {
        let bytes = unsafe { &*bytes };
        for i in 0..100 {
            dprint!("{} ", bytes[i]);
            if i % 10 == 9 {
                dprintln!();
            }
        }
        let elf = unsafe {
            let data: *const ElfHeader = bytes.as_ptr() as *const ElfHeader;
            &*(data)
        };
        dprintln!("{:?}", elf);
        if elf.magic != ELF_MAGIC {
            return Err(ElfError::InvalidMagic);
        }
        Ok(Elf { bytes, elf })
    }
    pub fn programs(&'a self) -> Programs<'a> {
        Programs {
            data: self.bytes,
            elf: self.elf,
            i: 0,
        }
    }
}

pub struct Program<'a> {
    pub virt_addr: paging::VirtAddr,
    pub mem_size: usize,
    pub phys_addr: paging::PhysAddr,
    pub flag: paging::Flag,
    pub data: &'a [u8],
    pub file_size: usize,
}

pub struct Programs<'a> {
    pub data: &'a [u8],
    pub elf: &'a ElfHeader,
    pub i: usize,
}

fn parse_elf_flag(flags: u32) -> paging::Flag {
    let mut flag = paging::Flag::VALID;

    if flags & 1 == 1 {
        flag |= paging::Flag::EXEC;
    }
    if (flags >> 1) & 1 == 1 {
        flag |= paging::Flag::WRITE;
    }
    if (flags >> 2) & 1 == 1 {
        flag |= paging::Flag::READ;
    }
    flag
}

impl<'a> Iterator for Programs<'a> {
    type Item = Program<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.i < (self.elf.phnum as usize) {
            dprintln!(
                "file place at {}",
                (self.elf.phoff as usize + self.i * self.elf.phentsize as usize)
            );
            let item = unsafe {
                &*((self.data.as_ptr() as usize
                    + self.elf.phoff as usize
                    + self.i * self.elf.phentsize as usize)
                    as *const ProgramHeader)
            };
            self.i += 1;
            let data = unsafe {
                &*(slice::from_raw_parts(
                    (self.data.as_ptr() as usize + item.offset as usize) as *const u8,
                    item.filesz as usize,
                ))
            };
            Some(Program {
                virt_addr: paging::VirtAddr::new(item.va),
                phys_addr: paging::PhysAddr::new(item.pa as u64),
                mem_size: item.memsz as usize,
                file_size: item.filesz as usize,
                flag: parse_elf_flag(item.flags),
                data,
            })
        } else {
            None
        }
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct ElfHeader {
    pub magic: u32,
    pub elf: [u8; 12],
    pub etype: u16,
    pub machine: u16,
    pub version: u32,
    pub entry: u32,
    pub phoff: u32,
    pub shoff: u32,
    pub flags: u32,
    pub size: u16,
    pub phentsize: u16,
    pub phnum: u16,
    pub shentsize: u16,
    pub shnum: u16,
    pub shstrndx: u16,
}

#[repr(C)]
pub struct ProgramHeader {
    pub ptype: u32,
    pub offset: u32,
    pub va: u32,
    pub pa: u32,
    pub filesz: u32,
    pub memsz: u32,
    pub flags: u32,
    pub align: u32,
}

#[allow(dead_code)]
#[repr(C)]
struct SectionHeader {
    name: u32,
    shtype: u32,
    flags: u32,
    addr: u32,
    offset: u32,
    size: u32,
    link: u32,
    info: u32,
    addralign: u32,
    entsize: u32,
}
