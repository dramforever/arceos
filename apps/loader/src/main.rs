#![cfg_attr(feature = "axstd", no_std)]
#![cfg_attr(feature = "axstd", no_main)]

#[cfg(feature = "axstd")]
use axstd::println;

const PLASH_START: usize = 0x22000000;

#[cfg_attr(feature = "axstd", no_mangle)]
fn main() {
    let apps_start = PLASH_START as *const u8;
    let apps_size = 32; // Dangerous!!! We need to get accurate size of apps.
    println!("Load payload ...");
    let code = unsafe { core::slice::from_raw_parts(apps_start, apps_size) };
    println!("content: {:#x}", bytes_to_usize(&code[..8]));
    println!("Load payload ok!");
}

#[inline]
fn bytes_to_usize(bytes: &[u8]) -> usize {
    usize::from_be_bytes(bytes.try_into().unwrap())
}
