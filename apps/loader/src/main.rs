#![cfg_attr(feature = "axstd", no_std)]
#![cfg_attr(feature = "axstd", no_main)]

#[cfg(feature = "axstd")]
use axstd::println;

const PLASH_START: usize = 0x22000000;
const PLASH_SIZE: usize = 32 << 20; // 32 MiB

#[cfg_attr(feature = "axstd", no_mangle)]
fn main() {
    let pflash = unsafe { core::slice::from_raw_parts(PLASH_START as *const u8, PLASH_SIZE) };

    for entry in cpio_reader::iter_files(pflash) {
        assert!(entry.mode().contains(cpio_reader::Mode::REGULAR_FILE));
        let code = entry.file();
        println!("Payload {} is {:02x?}", entry.name(), code);
    }
}
