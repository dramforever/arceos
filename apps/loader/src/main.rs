#![cfg_attr(feature = "axstd", no_std)]
#![cfg_attr(feature = "axstd", no_main)]

#[cfg(feature = "axstd")]
use axstd::println;

const PLASH_START: usize = 0x22000000;
const PLASH_SIZE: usize = 32 << 20; // 32 MiB

const RUN_START: usize = 0xffff_ffc0_8010_0000;

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

fn abi_terminate() {
    axstd::process::exit(0);
}

unsafe fn call_code(addr: usize) {
    core::arch::asm!("jalr {}", in(reg) addr);
}

#[cfg_attr(feature = "axstd", no_mangle)]
fn main() {
    register_abi(SYS_HELLO, abi_hello as usize);
    register_abi(SYS_PUTCHAR, abi_putchar as usize);
    register_abi(SYS_TERMINATE, abi_terminate as usize);

    use core::slice::{from_raw_parts, from_raw_parts_mut};
    let pflash = unsafe { from_raw_parts(PLASH_START as *const u8, PLASH_SIZE) };

    for entry in cpio_reader::iter_files(pflash) {
        assert!(entry.mode().contains(cpio_reader::Mode::REGULAR_FILE));
        println!("Running {}", entry.name());

        let code = entry.file();

        let run_code = unsafe { from_raw_parts_mut(RUN_START as *mut u8, code.len()) };
        run_code.copy_from_slice(code);

        unsafe {
            call_code(RUN_START);
        }
    }
}
