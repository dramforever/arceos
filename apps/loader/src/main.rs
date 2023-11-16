#![cfg_attr(feature = "axstd", no_std)]
#![cfg_attr(feature = "axstd", no_main)]

#[cfg(feature = "axstd")]
use axstd::println;

const PLASH_START: usize = 0x22000000;
const PLASH_SIZE: usize = 32 << 20; // 32 MiB

const RUN_START: usize = 0xffff_ffc0_8010_0000;

unsafe fn call_code(addr: usize) {
    core::arch::asm!("jalr {}", in(reg) addr);
}

#[cfg_attr(feature = "axstd", no_mangle)]
fn main() {
    use core::slice::{from_raw_parts, from_raw_parts_mut};
    let pflash = unsafe { from_raw_parts(PLASH_START as *const u8, PLASH_SIZE) };

    for entry in cpio_reader::iter_files(pflash) {
        assert!(entry.mode().contains(cpio_reader::Mode::REGULAR_FILE));
        println!("Running {}", entry.name());

        let code = entry.file();

        let run_code = unsafe { from_raw_parts_mut(RUN_START as *mut u8, code.len()) };
        run_code.copy_from_slice(code);

        unsafe { call_code(RUN_START); }
    }
}
