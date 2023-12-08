use core::slice;

use axstd::io::{stdout, Write};
// use axstd::println;

use crate::abi::*;

fn writev(fd: c_int, iov: *const c_iovec, iovcnt: c_int) -> Result<c_ssize_t, c_ssize_t> {
    if !(0..=2).contains(&fd) {
        return Err(22);
    }

    let iovcnt: usize = iovcnt.try_into().ok().ok_or(22_isize)?;

    let iovs = unsafe { slice::from_raw_parts(iov, iovcnt) };

    let mut total: isize = 0;

    for iov in iovs {
        let buf = unsafe { slice::from_raw_parts(iov.iov_base, iov.iov_len) };
        match stdout().lock().write(buf) {
            Ok(bytes) => {
                total += bytes as isize;
                if bytes != buf.len() {
                    return Ok(total);
                }
            }
            Err(_) => {
                if total == 0 {
                    return Err(5);
                } else {
                    return Ok(total);
                }
            }
        }
    }

    Ok(total)
}

fn brk(new_brk: usize) -> usize {
    let mut user = crate::USER.borrow_mut();
    if new_brk >= user.brk_min {
        if new_brk > user.brk_max {
            let old_max = user.brk_max;
            let new_max = new_brk.next_multiple_of(crate::PAGE_SIZE);
            unsafe {
                user.map_new(old_max, new_max - old_max);
            }
            user.brk_max = new_max;
        }

        user.brk = new_brk;
    }

    // println!("brk {:#x} -> {:#x}", new_brk, user.brk);

    user.brk
}

fn mmap(
    addr: usize,
    length: c_size_t,
    prot: c_int,
    flags: c_int,
    fd: c_int,
    offset: c_off_t,
) -> usize {
    #![allow(unused_variables)]
    // println!("mmap {addr:#x} {length:#x} {prot:#x} {flags:#x} {fd:#x} {offset:#x}");
    0 // STUB
}

fn mprotect(addr: usize, length: c_size_t, prot: c_int) -> c_int {
    #![allow(unused_variables)]
    // println!("mprotect {addr:#x} {length:#x} {prot:#x}");
    0 // STUB
}

#[no_mangle]
#[allow(unused_variables)]
#[allow(clippy::too_many_arguments)]
pub unsafe fn axmusl_syscall_handler(
    _: &[usize; 2],
    n: isize,
    a0: isize,
    a1: isize,
    a2: isize,
    a3: isize,
    a4: isize,
    a5: isize,
) -> isize {
    // FIXME: Syscall numbers

    match n {
        96 => 1, // set_tid_address
        66 => writev(a0 as _, a1 as _, a2 as _).unwrap_or_else(|e| -e),
        214 => brk(a0 as _) as _,
        222 => mmap(a0 as _, a1 as _, a2 as _, a3 as _, a4 as _, a5 as _) as _,
        226 => mprotect(a0 as _, a1 as _, a2 as _) as _,
        29 => -22, // ioctl
        57 => 0,   // close
        94 => panic!("exit"),
        _ => panic!("syscall {n} not implemented"),
    }
}
