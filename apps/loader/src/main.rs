#![cfg_attr(feature = "axstd", no_std)]
#![cfg_attr(feature = "axstd", no_main)]

#[cfg(feature = "axstd")]
use axstd::println;
use elf::{endian::AnyEndian, ElfBytes};

const PLASH_START: usize = 0x22000000;
const PLASH_SIZE: usize = 32 << 20; // 32 MiB

const RUN_START: usize = 0xffff_ffc0_8010_0000;
const RUN_SIZE: usize = 1 << 20; // 1 MiB

const SYS_HELLO: usize = 1;
const SYS_PUTCHAR: usize = 2;
const SYS_TERMINATE: usize = 3;

static mut ABI_TABLE: [usize; 16] = [0; 16];

fn register_abi(num: usize, handle: usize) {
    unsafe {
        ABI_TABLE[num] = handle;
    }
}

fn abi_hello() {
    println!("[ABI:Hello] Hello, Apps!")
}

fn abi_putchar(c: char) {
    println!("[ABI:Print] {c}");
}

fn abi_terminate() -> ! {
    axstd::process::exit(0);
}

unsafe fn call_code(addr: usize) {
    core::arch::asm!(
        "jalr {}",
        in(reg) addr,
        in("a7") &ABI_TABLE,
    );
}

unsafe fn load_code(data: &[u8]) -> usize {
    let elf = <ElfBytes<AnyEndian>>::minimal_parse(data).expect("Invalid ELF");
    let phdrs = elf.segments().expect("ELF should have segments");
    for phdr in phdrs {
        if phdr.p_type == elf::abi::PT_LOAD {
            assert!(phdr.p_filesz <= phdr.p_memsz);
            assert!(phdr.p_vaddr >= RUN_START as u64,);
            assert!(phdr.p_vaddr.saturating_add(phdr.p_memsz) <= (RUN_START + RUN_SIZE) as u64);
            let region = unsafe {
                core::slice::from_raw_parts_mut(
                    phdr.p_vaddr as usize as *mut u8,
                    phdr.p_memsz as usize,
                )
            };
            let (filled, zeroed) = region.split_at_mut(phdr.p_filesz as usize);
            filled.copy_from_slice(&data[phdr.p_offset as usize..][..phdr.p_filesz as usize]);
            zeroed.fill(0);
        }
    }
    let entry = elf.ehdr.e_entry as usize;
    assert!((RUN_START..RUN_START + RUN_SIZE).contains(&entry));
    entry
}

#[cfg_attr(feature = "axstd", no_mangle)]
fn main() {
    register_abi(SYS_HELLO, abi_hello as usize);
    register_abi(SYS_PUTCHAR, abi_putchar as usize);
    register_abi(SYS_TERMINATE, abi_terminate as usize);

    let pflash = unsafe { core::slice::from_raw_parts(PLASH_START as *const u8, PLASH_SIZE) };

    for entry in cpio_reader::iter_files(pflash) {
        assert!(entry.mode().contains(cpio_reader::Mode::REGULAR_FILE));
        println!("Running {}", entry.name());

        let code = entry.file();

        unsafe {
            let entry = load_code(code);
            call_code(entry);
        }
    }
}
