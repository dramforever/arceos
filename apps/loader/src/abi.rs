#![allow(non_camel_case_types)]

pub type c_void_p = *mut u8;
pub type c_int = i32;

pub type c_size_t = usize;
pub type c_ssize_t = isize;

pub type c_off_t = isize;

#[repr(C)]
pub struct c_iovec {
    pub iov_base: c_void_p,
    pub iov_len: c_size_t,
}
