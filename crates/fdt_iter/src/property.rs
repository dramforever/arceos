use core::ffi::CStr;
use core::fmt::{self, Debug};

use crate::debug_iter::*;

/// Parse a basic value of type `<stringlist>`
///
/// ```
/// # use fdt_iter::property::*;
/// # use cstr::cstr;
/// let compatible = b"fsl,mpc8641\0ns16550\0";
/// let compatible_list = [cstr!("fsl,mpc8641"), cstr!("ns16550")];
/// assert!(string_list(compatible).unwrap().eq(compatible_list));
/// ```
///
/// ```
/// # use fdt_iter::property::*;
/// let invalid = b"terminated\0not-terminated";
/// assert!(string_list(invalid).is_none());
/// ```
pub fn string_list(data: &[u8]) -> Option<impl Iterator<Item = &CStr> + Clone + Debug> {
    // Either empty or ends with a NUL
    let valid = data.last().map_or(true, |&x| x == 0);

    valid.then_some(
        data.split_inclusive(|x| *x == 0)
            .map(|x| CStr::from_bytes_with_nul(x).unwrap()),
    )
}

/// Parse a basic value of type `<string>`
///
/// ```
/// # use fdt_iter::property::*;
/// # use cstr::cstr;
/// let status = b"okay\0";
/// assert_eq!(string(status), Some(cstr!("okay")));
/// ```
///
/// ```
/// # use fdt_iter::property::*;
/// let invalid = b"two-strings\0is-invalid\0";
/// assert!(string(invalid).is_none());
/// ```
pub fn string(data: &[u8]) -> Option<&CStr> {
    CStr::from_bytes_with_nul(data).ok()
}

/// Parse a basic value of type `<u32>`
///
/// ```
/// # use fdt_iter::property::*;
/// let address_cells: &[u8] = &[0, 0, 0, 2];
/// assert_eq!(u32(address_cells), Some(2));
/// ```
pub fn u32(data: &[u8]) -> Option<u32> {
    let data = data.try_into().ok()?;
    Some(u32::from_be_bytes(data))
}

/// Parse a big endian unsigned integer of any byte length `0..=8`.
///
/// If length is `0`, returns `0`. If length is too long, returns `None`.
///
/// Suitable for a property that's either `<u32>` or `<u64>`.
///
/// ```
/// # use fdt_iter::property::*;
/// # use hex_literal::hex;
/// let zero: &[u8] = &[];
/// assert_eq!(unsigned(zero), Some(0));
///
/// let clock_frequency: &[u8] = &hex!("3b9aca00");
/// assert_eq!(unsigned(clock_frequency), Some(1_000_000_000u64)); // 1 GHz
///
/// let clock_frequency: &[u8] = &hex!("00000001 2a05f200");
/// assert_eq!(unsigned(clock_frequency), Some(5_000_000_000u64)); // 5 GHz
/// ```
pub fn unsigned(data: &[u8]) -> Option<u64> {
    let valid = data.len() <= 8;

    if !valid {
        panic!("{data:?}");
    }

    valid.then(|| {
        let mut thing = [0; 8];
        thing[8 - data.len()..8].copy_from_slice(data);
        u64::from_be_bytes(thing)
    })
}

/// Parse a list of `<u32>`
///
/// ```
/// # use fdt_iter::property::*;
/// # use hex_literal::hex;
/// let interrupts: &[u8] = &hex!("00000002 00000003 00000004");
/// assert!(u32_list(interrupts).unwrap().eq([2, 3, 4]));
/// ```
pub fn u32_list(data: &[u8]) -> Option<impl Iterator<Item = u32> + Clone + Debug + '_> {
    let chunks = data.chunks_exact(4);
    let valid = chunks.remainder().is_empty();
    valid.then_some(chunks.map(|x| u32::from_be_bytes(x.try_into().unwrap())))
}

/// Split a `reg` property into addresses and sizes
pub fn reg_list_raw(
    data: &[u8],
    address_cells: usize,
    size_cells: usize,
) -> Option<impl Iterator<Item = (&[u8], &[u8])> + Clone + Debug> {
    let chunks = data.chunks_exact(4 * (address_cells + size_cells));
    let valid = chunks.remainder().is_empty();

    valid.then_some(
        chunks
            .map(move |chunk| chunk.split_at(4 * address_cells))
            .debug(),
    )
}

/// Parse a simple address-based `reg` property as `(addr, size)` pairs
///
/// `reg_list` Only works with values of at most 64-bit, i.e. `#*-cells <= 2`.
/// If you have values represented by more than 2 cells, such as for PCI nodes,
/// use [`reg_list_raw`] instead.
///
/// If `#size-cells == 0`, all sizes given will be `0`.
///
/// # Arguments
///
/// * `address_cells`: `#address-cells` of parent node
/// * `size_cells`: `#size-cells` of parent node
///
/// # Panics
///
/// Panics if `address_cells` is outside `1..=2` or `size_cells` is outside `0..=2`.
///
/// # Examples
///
/// ```
/// # use fdt_iter::property::*;
/// # use hex_literal::hex;
/// let reg: &[u8] = &hex!("
///     // Start at 0, size 2GiB
///     00000000 00000000 00000000 80000000
///     // Start at 4GiB, size 4GiB
///     00000001 00000000 00000001 00000000
/// ");
///
/// let expected = [(0, 0x8000_0000), (0x1_0000_0000, 0x1_0000_0000)];
/// assert!(reg_list(reg, 2, 2).unwrap().eq(expected));
/// ```
pub fn reg_list(
    data: &[u8],
    address_cells: usize,
    size_cells: usize,
) -> Option<impl Iterator<Item = (u64, u64)> + Clone + Debug + '_> {
    assert!(
        (1..=2).contains(&address_cells),
        "#address-cells must be in 1..=2"
    );
    assert!(
        (0..=2).contains(&size_cells),
        "#size-cells must be in 0..=2"
    );

    let iter = reg_list_raw(data, address_cells, size_cells)?;
    let iter = iter
        .map(|(addr, size)| (unsigned(addr).unwrap(), unsigned(size).unwrap()))
        .debug();
    Some(iter)
}

/// Value of the standard property `status`
#[derive(Clone, PartialEq, Eq)]
pub enum Status<'a> {
    /// `okay`
    Okay,
    /// `disabled`
    Disabled,
    /// `reserved`
    Reserved,
    /// `fail` or `fail-sss`
    Fail(Option<&'a CStr>),
}

impl Debug for Status<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Okay => write!(f, "okay"),
            Self::Disabled => write!(f, "disabled"),
            Self::Reserved => write!(f, "reserved"),
            Self::Fail(None) => write!(f, "fail"),
            Self::Fail(Some(reason)) => write!(f, "fail-{reason:?}"),
        }
    }
}

impl<'a> Status<'a> {
    /// Parse the standard property `status`
    ///
    /// ```
    /// # use fdt_iter::property::*;
    /// let status = b"okay\0";
    /// assert_eq!(Status::from_bytes(status), Some(Status::Okay));
    /// ```
    pub fn from_bytes(data: &'a [u8]) -> Option<Self> {
        let data = string(data)?;

        Some(match data.to_bytes() {
            b"okay" => Self::Okay,
            b"disabled" => Self::Disabled,
            b"reserved" => Self::Reserved,
            b"fail" => Self::Fail(None),
            x if x.starts_with(b"fail-") => Self::Fail(Some(&data[b"fail-".len()..])),
            _ => return None,
        })
    }

    /// Returns true if the status is `okay`
    ///
    /// ```
    /// # use fdt_iter::property::*;
    /// assert!(Status::Okay.is_okay());
    /// assert!(!Status::Disabled.is_okay());
    /// ```
    pub fn is_okay(&self) -> bool {
        matches!(self, Self::Okay)
    }
}
