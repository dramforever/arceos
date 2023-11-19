#![cfg_attr(feature = "axstd", no_std)]
#![cfg_attr(feature = "axstd", no_main)]

#[cfg(feature = "axstd")]
use axstd::println;
use elf::{endian::AnyEndian, ElfBytes};

const PLASH_START: usize = 0x22000000;
const PLASH_SIZE: usize = 32 << 20; // 32 MiB

const RUN_START: usize = 0x4000_0000;
const RUN_SIZE: usize = 16 << 12; // 16 pages

const SYS_HELLO: usize = 1;
const SYS_PUTCHAR: usize = 2;
const SYS_TERMINATE: usize = 3;

static mut ABI_TABLE: [usize; 16] = [0; 16];

#[link_section = ".data.app_page_table"]
static mut APP_PT_SV39: [u64; 512] = [0; 512];

#[link_section = ".data.app_page_table"]
static mut APP_PT_SV39_1: [u64; 512] = [0; 512];

#[link_section = ".data.app_page_table"]
static mut APP_PT_SV39_0: [u64; 512] = [0; 512];

unsafe fn init_app_page_table() {
    // 0x8000_0000..0xc000_0000, VRWX_GAD, 1G block
    APP_PT_SV39[2] = (0x80000 << 10) | 0xef;
    // 0xffff_ffc0_8000_0000..0xffff_ffc0_c000_0000, VRWX_GAD, 1G block
    APP_PT_SV39[0x102] = (0x80000 << 10) | 0xef;
    // 0x0000_0000..0x4000_0000, VRWX_GAD, 1G block
    APP_PT_SV39[0] = (0x00000 << 10) | 0xef;

    let pt_1_pa = APP_PT_SV39_1.as_ptr() as usize - axconfig::PHYS_VIRT_OFFSET;
    let pt_0_pa = APP_PT_SV39_0.as_ptr() as usize - axconfig::PHYS_VIRT_OFFSET;

    // For App aspace!
    APP_PT_SV39[1] = (pt_1_pa as u64 >> 12 << 10) | 0x21;
    APP_PT_SV39_1[0] = (pt_0_pa as u64 >> 12 << 10) | 0x21;

    use riscv::register::satp;
    let page_table_root = APP_PT_SV39.as_ptr() as usize - axconfig::PHYS_VIRT_OFFSET;
    satp::set(satp::Mode::Sv39, 0, page_table_root >> 12);
    riscv::asm::sfence_vma_all();
}

unsafe fn switch_app_aspace(addr: usize) {
    for a in (0..RUN_SIZE).step_by(1 << 12) {
        APP_PT_SV39_0[a >> 12] = ((addr + a) as u64 >> 12 << 10) | 0xef;
    }
    riscv::asm::sfence_vma_all();
}

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

    unsafe {
        init_app_page_table();
    };

    let mut pa = 0x8010_0000;

    for entry in cpio_reader::iter_files(pflash) {
        unsafe {
            switch_app_aspace(pa);
        }

        assert!(entry.mode().contains(cpio_reader::Mode::REGULAR_FILE));
        println!("Running {} at physical address {:#x}", entry.name(), pa);

        let code = entry.file();

        unsafe {
            let entry = load_code(code);
            call_code(entry);
        }

        pa += RUN_SIZE;
    }
}
