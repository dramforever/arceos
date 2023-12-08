#![no_std]
#![no_main]
#![feature(thread_local)]

extern crate alloc;

use core::{arch::asm, cell::RefCell, mem::size_of, slice};

use alloc::{borrow::ToOwned, vec::Vec};
use axconfig::PHYS_VIRT_OFFSET;
use axruntime::dtb_phys;
use axstd::println;
use elf::{abi::*, endian::NativeEndian, ElfBytes};
use fdt::Fdt;

mod abi;
mod syscall;

mod elf_consts {
    #![allow(unused)]

    pub const AT_NULL: usize = 0;
    pub const AT_IGNORE: usize = 1;
    pub const AT_EXECFD: usize = 2;
    pub const AT_PHDR: usize = 3;
    pub const AT_PHENT: usize = 4;
    pub const AT_PHNUM: usize = 5;
    pub const AT_PAGESZ: usize = 6;
    pub const AT_BASE: usize = 7;
    pub const AT_FLAGS: usize = 8;
    pub const AT_ENTRY: usize = 9;
    pub const AT_NOTELF: usize = 10;
    pub const AT_UID: usize = 11;
    pub const AT_EUID: usize = 12;
    pub const AT_GID: usize = 13;
    pub const AT_EGID: usize = 14;
    pub const AT_CLKTCK: usize = 17;
}

fn fdt_bytes() -> Vec<u8> {
    let fdt_ptr = (dtb_phys() + PHYS_VIRT_OFFSET) as *const u8;
    let fdt_total_size = unsafe { Fdt::from_ptr(fdt_ptr).unwrap() }.total_size();
    let fdt = unsafe { slice::from_raw_parts(fdt_ptr, fdt_total_size) };
    fdt.to_owned()
}

fn initrd(fdt: &Fdt) -> Vec<u8> {
    let chosen = fdt.find_node("/chosen").unwrap();
    let (initrd_start, initrd_end) = (|| {
        Some((
            chosen.property("linux,initrd-start")?.as_usize()?,
            chosen.property("linux,initrd-end")?.as_usize()?,
        ))
    })()
    .expect("Missing or invalid initrd");
    let initrd = unsafe {
        slice::from_raw_parts(
            (initrd_start + PHYS_VIRT_OFFSET) as *const u8,
            initrd_end - initrd_start,
        )
    };
    initrd.to_owned()
}

const PAGE_SIZE: usize = 4096;
const SIZE_BYTES: usize = size_of::<usize>();

type Page = [usize; PAGE_SIZE / SIZE_BYTES];

fn alloc_one_page() -> *mut Page {
    let va = axalloc::global_allocator()
        .alloc_pages(1, PAGE_SIZE)
        .unwrap();
    let res = va as *mut Page;
    unsafe { (*res).fill(0) }
    res
}

const EXEC_BASE: usize = 0x1000_0000;
const DYLD_BASE: usize = 0x10_0000_0000;
const STACK_TOP: usize = 0x3f_0000_0000;
const STACK_SIZE: usize = 8 << 20;

struct User {
    pgroot: *mut Page,
    brk: usize,
    brk_min: usize,
    brk_max: usize,
}

fn sfence_vma() {
    unsafe {
        asm!("sfence.vma", options(nomem, nostack));
    }
}

#[allow(clippy::unusual_byte_groupings)]
impl User {
    const fn new() -> User {
        Self {
            pgroot: core::ptr::null_mut(),
            brk: 0,
            brk_min: 0,
            brk_max: 0,
        }
    }

    unsafe fn make_current(&mut self) {
        let satp = (8 << 60) | ((self.pgroot as usize - PHYS_VIRT_OFFSET) >> 12);
        unsafe {
            asm!("csrw satp, {}", in(reg) satp, options(nomem, nostack));
        }
        sfence_vma();
    }

    unsafe fn map_one(&mut self, va: usize, pa: usize, level: usize) {
        if self.pgroot.is_null() {
            self.pgroot = alloc_one_page();
        }

        let mut node = unsafe { &mut *self.pgroot };

        for l in (level + 1..=2).rev() {
            let vpn = (va >> (12 + l * 9)) & ((1 << 9) - 1);
            let pte = &mut node[vpn];

            if *pte & 1 == 0 {
                let p = alloc_one_page() as usize - PHYS_VIRT_OFFSET;
                *pte = (p >> 2) | 0b00_1_0_000_1; // -- g - --- v
            }

            let node_pa = *pte >> 10 << 12;
            node = unsafe { &mut *((node_pa + PHYS_VIRT_OFFSET) as *mut Page) };
        }

        let vpn = (va >> (12 + level * 9)) & ((1 << 9) - 1);
        let pte = &mut node[vpn];
        *pte = (pa >> 2) | 0b11_1_0_111_1; // da g - xwr v
    }

    unsafe fn map_new(&mut self, va: usize, len: usize) {
        assert!(len % PAGE_SIZE == 0);
        assert!(va % PAGE_SIZE == 0);

        for off in (0..len).step_by(4096) {
            let page = alloc_one_page();
            let pa = page as usize - PHYS_VIRT_OFFSET;
            self.map_one(va + off, pa, 0);
        }
    }
}

extern "C" {
    fn enter_program(sp: usize, entry: usize) -> !;
}

core::arch::global_asm!(include_str!("asm.s"));

#[thread_local]
static USER: RefCell<User> = RefCell::new(User::new());

#[no_mangle]
fn main() {
    let fdt = fdt_bytes();
    let fdt = Fdt::new(&fdt).unwrap();
    let initrd = initrd(&fdt);
    let dyld = cpio_reader::iter_files(&initrd)
        .find(|f| f.name() == "ld.so")
        .expect("No ld.so");
    let main = cpio_reader::iter_files(&initrd)
        .find(|f| f.name() != "ld.so")
        .expect("No main");
    for f in [&dyld, &main] {
        println!(
            "{:?} len = {}, mode = 0o{:o}",
            f.name(),
            f.file().len(),
            f.mode().bits()
        );
    }

    let pc: usize;
    unsafe {
        asm!("auipc {}, 0", out(reg) pc, options(nomem, nostack));
    }
    let off = (pc - PHYS_VIRT_OFFSET) & !((1 << (12 + 9 * 2)) - 1);

    let main_elf = elf::ElfBytes::<NativeEndian>::minimal_parse(main.file()).unwrap();
    let dyld_elf = elf::ElfBytes::<NativeEndian>::minimal_parse(dyld.file()).unwrap();

    {
        let mut user = USER.borrow_mut();

        unsafe {
            user.map_one(off, off, 2);
            user.map_one(PHYS_VIRT_OFFSET + off, off, 2);
            user.map_new(STACK_TOP - STACK_SIZE, STACK_SIZE);
            user.make_current();
        }

        user.brk = map_elf(&mut user, &main_elf, main.file(), EXEC_BASE);
        user.brk_max = user.brk;
        user.brk_min = user.brk;
        map_elf(&mut user, &dyld_elf, dyld.file(), DYLD_BASE);
    }

    let mut sp = STACK_TOP as *mut usize;

    let push = |sp: &mut *mut usize, value: usize| {
        *sp = unsafe { (*sp).sub(1) };
        unsafe {
            **sp = value;
        }
    };

    let pushstr = |sp: &mut *mut usize, str: &[u8]| {
        *sp = unsafe { (*sp).sub((str.len() + 1).next_multiple_of(SIZE_BYTES)) };
        let dest = unsafe { slice::from_raw_parts_mut(*sp as *mut u8, str.len()) };
        dest.copy_from_slice(str);
    };

    {
        use elf_consts::*;
        pushstr(&mut sp, main.name().as_bytes());
        let argv0 = sp;

        pushstr(&mut sp, b"--help");
        let argv1 = sp;

        push(&mut sp, 0);
        push(&mut sp, AT_NULL);

        push(&mut sp, main_elf.ehdr.e_phoff as usize + EXEC_BASE);
        push(&mut sp, AT_PHDR);

        push(&mut sp, main_elf.ehdr.e_phentsize as usize);
        push(&mut sp, AT_PHENT);

        push(&mut sp, main_elf.ehdr.e_phnum as usize);
        push(&mut sp, AT_PHNUM);

        push(&mut sp, PAGE_SIZE);
        push(&mut sp, AT_PAGESZ);

        push(&mut sp, DYLD_BASE);
        push(&mut sp, AT_BASE);

        push(&mut sp, main_elf.ehdr.e_entry as usize + EXEC_BASE);
        push(&mut sp, AT_ENTRY);

        push(&mut sp, 0);

        push(&mut sp, 0);
        push(&mut sp, argv1 as usize);
        push(&mut sp, argv0 as usize);
        push(&mut sp, 2);
    }

    let final_sp = sp;
    println!("final_sp = {:#x}", final_sp as usize);


    println!("=== Entering user program ===");

    let entry = dyld_elf.ehdr.e_entry as usize + DYLD_BASE;
    unsafe {
        enter_program(final_sp as usize, entry);
    }
}

fn map_elf(user: &mut User, ef: &ElfBytes<NativeEndian>, data: &[u8], base: usize) -> usize {
    assert!(ef.ehdr.e_type == ET_DYN);

    let mut max_addr = 0;

    for phdr in ef.segments().unwrap() {
        if phdr.p_type != PT_LOAD {
            continue;
        }

        let va = phdr.p_vaddr as usize + base;
        let off = phdr.p_offset as usize;
        let filesz = phdr.p_filesz as usize;
        let memsz = phdr.p_memsz as usize;

        let vabase = va & !(PAGE_SIZE - 1);
        let vasize = (va - vabase + memsz).next_multiple_of(PAGE_SIZE);
        println!("{vabase:#x} + {vasize:#x}");
        unsafe {
            user.map_new(vabase, vasize);
            sfence_vma();
        }
        let dest = &mut unsafe { slice::from_raw_parts_mut(va as *mut u8, filesz) };
        dest.copy_from_slice(&data[off..][..filesz]);
        max_addr = max_addr.max(vabase + vasize);
    }

    max_addr
}
