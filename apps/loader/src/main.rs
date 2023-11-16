#![feature(asm_const)]
#![cfg_attr(feature = "axstd", no_std)]
#![cfg_attr(feature = "axstd", no_main)]

#[cfg(feature = "axstd")]
use axstd::println;

const PLASH_START: usize = 0x22000000;

#[cfg_attr(feature = "axstd", no_mangle)]
fn main() {
    let load_start = PLASH_START as *const u8;
    let load_size = 32; // Dangerous!!! We need to get accurate size of apps.

    println!("Load payload ...");

    let load_code = unsafe { core::slice::from_raw_parts(load_start, load_size) };

    println!(
        "load code {:?}; address [{:?}]",
        load_code,
        load_code.as_ptr()
    );

    // app running aspace
    // SBI(0x80000000) -> App <- Kernel(0x80200000)
    // 0xffff_ffc0_0000_0000
    const RUN_START: usize = 0xffff_ffc0_8010_0000;
    let run_code = unsafe { core::slice::from_raw_parts_mut(RUN_START as *mut u8, load_size) };

    run_code.copy_from_slice(load_code);

    println!("run code {:?}; address [{:?}]", run_code, run_code.as_ptr());

    println!("Execute app ...");
    // execute app
    unsafe {
        core::arch::asm!("
            li t2, {run_start}
            jalr t2
            j .
            ",
            run_start = const RUN_START,
        );
    }
}
