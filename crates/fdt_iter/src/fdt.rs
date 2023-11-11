use core::fmt;
use core::{mem::size_of, ops::Range, slice};
use zerocopy::{BigEndian, FromBytes, U32};

use crate::node::Node;
pub(crate) use crate::op::*;

#[derive(Clone, Copy, FromBytes)]
#[allow(unused)]
#[repr(C)]
struct Header {
    magic: U32<BigEndian>,
    totalsize: U32<BigEndian>,
    off_dt_struct: U32<BigEndian>,
    off_dt_strings: U32<BigEndian>,
    off_mem_rsvmap: U32<BigEndian>,
    version: U32<BigEndian>,
    last_comp_version: U32<BigEndian>,
    boot_cpuid_phys: U32<BigEndian>,
    size_dt_strings: U32<BigEndian>,
    size_dt_struct: U32<BigEndian>,
}

impl Header {
    fn from_bytes(fdt: &[u8]) -> Option<Self> {
        Header::read_from_prefix(fdt)
    }

    fn valid_magic(&self) -> bool {
        self.magic.get() == 0xd00d_feed
    }
}

/// A flattened devicetree
///
/// See [crate level documentation][crate] for an introduction.
pub struct Fdt<'a> {
    raw: &'a [u8],
    struct_range: Range<usize>,
    strings_range: Range<usize>,
}

fn fix_strings_range(bytes: &[u8], mut range: Range<usize>) -> Range<usize> {
    while bytes[range.clone()].last().map_or(false, |&x| x != 0) {
        range.end -= 1;
    }

    range
}

fn make_range(start: usize, len: usize) -> Option<Range<usize>> {
    start.checked_add(len).map(|end| start..end)
}

/// A flattened devicetree validation error
pub struct FdtError {
    message: &'static str,
    offset: Option<usize>,
}

impl FdtError {
    fn from_message(message: &'static str) -> Self {
        Self {
            message,
            offset: None,
        }
    }

    fn from_message_offset(message: &'static str, offset: usize) -> Self {
        Self {
            message,
            offset: Some(offset),
        }
    }
}

impl fmt::Debug for FdtError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(offset) = self.offset {
            write!(f, "FDT error, at byte {offset}: {}", self.message)
        } else {
            write!(f, "FDT header error: {}", self.message)
        }
    }
}

fn check<E>(check: bool, err: E) -> Result<(), E> {
    check.then_some(()).ok_or(err)
}

impl<'a> Fdt<'a> {
    /// Create and validate a flattened devicetree from `bytes`
    pub fn from_bytes(bytes: &'a [u8]) -> Result<Self, FdtError> {
        let header =
            Header::from_bytes(bytes).ok_or(FdtError::from_message("Not enough bytes for FDT"))?;

        // If magic is wrong stop here and don't continue
        check(
            header.valid_magic(),
            FdtError::from_message("Invalid FDT magic"),
        )?;

        let total_size = header.totalsize.get() as usize;

        check(
            total_size <= bytes.len(),
            FdtError::from_message("FDT totalsize more than available data"),
        )?;

        let struct_start = header.off_dt_struct.get() as usize;
        let struct_len = header.size_dt_struct.get() as usize;
        let strings_start = header.off_dt_strings.get() as usize;
        let strings_len = header.size_dt_strings.get() as usize;

        let struct_range = make_range(struct_start, struct_len).ok_or(FdtError::from_message(
            "Bad structure block, range overflows",
        ))?;

        let strings_range = make_range(strings_start, strings_len)
            .ok_or(FdtError::from_message("Bad strings block, range overflows"))?;

        let strings_range = fix_strings_range(bytes, strings_range);

        let res = Fdt {
            raw: bytes,
            struct_range,
            strings_range,
        };

        res.validate()?;
        Ok(res)
    }

    /// Create and validate a flattened devicetree from memory at `ptr`
    ///
    /// Trust the `totalsize` field of the FDT header.
    ///
    /// # Safety
    ///
    /// A valid FDT must indeed exist at `ptr`. Although this function tries to
    /// validate the FDT, it is the caller that must ensure the memory safety of
    /// `ptr`.
    pub unsafe fn from_ptr(ptr: *const u8) -> Result<Self, FdtError> {
        // SAFETY: Ensured by caller
        let header = unsafe { slice::from_raw_parts(ptr, size_of::<Header>()) };
        let header = Header::from_bytes(header).unwrap();

        // If magic is wrong stop here and don't continue
        check(
            header.valid_magic(),
            FdtError::from_message("Invalid FDT magic"),
        )?;

        let len = header.totalsize.get() as usize;

        // SAFETY: Ensured by caller
        Self::from_bytes(unsafe { slice::from_raw_parts(ptr, len) })
    }

    fn header(&self) -> Header {
        Header::from_bytes(self.raw).unwrap()
    }

    fn validate(&self) -> Result<(), FdtError> {
        (|| -> Result<(), &'static str> {
            check(
                self.struct_range.start <= self.raw.len(),
                "Structure block start out of range",
            )?;
            check(
                self.struct_range.end <= self.raw.len(),
                "Structure block end out of range",
            )?;
            check(
                self.strings_range.start <= self.raw.len(),
                "Strings block start out of range",
            )?;
            check(
                self.strings_range.end <= self.raw.len(),
                "Strings block end out of range",
            )?;
            Ok(())
        })()
        .map_err(FdtError::from_message)?;

        let mut input = self.struct_block();
        let mut depth: usize = 0;

        loop {
            let offset = self.strings_range.end - input.len();
            let msg = |m| FdtError::from_message_offset(m, offset);

            let result;
            (input, result) = OpRaw::parse(input).map_err(|_| msg("Bad FDT token"))?;

            if let Some(op) = result {
                match op {
                    OpRaw::BeginNode { name: _ } => {
                        depth += 1;
                    }
                    OpRaw::EndNode => {
                        depth = depth
                            .checked_sub(1)
                            .ok_or(msg("Unexpected FDT_END_NODE, no matching FDT_BEGIN_NODE"))?;
                    }
                    OpRaw::Prop {
                        name_offset,
                        value: _,
                    } => check(
                        name_offset < self.strings_range.len(),
                        msg("Bad FDT_PROP, name is out of range or not NUL-terminated"),
                    )?,
                }
            } else {
                check(depth == 0, msg("Unexpected FDT_END, missing FDT_END_NODE"))?;
                break;
            }
        }

        Ok(())
    }

    fn struct_block(&self) -> &[u8] {
        &self.raw[self.struct_range.clone()]
    }

    fn strings_block(&self) -> &[u8] {
        &self.raw[self.strings_range.clone()]
    }

    /// Get a node by its byte offset
    pub fn node_from_offset(&self, offset: usize) -> Node {
        assert!(self.struct_range.contains(&offset));
        let iter = OpIter {
            fdt: self,
            remain: &self.struct_block()[offset - self.struct_range.start..],
        };
        assert!(matches!(iter.peek(), Some(Op::BeginNode { .. })));
        Node(iter)
    }

    /// Get the root node
    pub fn root(&self) -> Node {
        self.node_from_offset(self.header().off_dt_struct.get() as usize)
    }
}

#[derive(Clone)]
pub(crate) struct OpIter<'a> {
    fdt: &'a Fdt<'a>,
    remain: &'a [u8],
}

impl<'a> OpIter<'a> {
    pub fn offset(&self) -> usize {
        self.fdt.struct_range.end - self.remain.len()
    }

    pub fn peek(&self) -> Option<Op<'a>> {
        let mut copy = self.clone();
        copy.next()
    }
}

impl<'a> Iterator for OpIter<'a> {
    type Item = Op<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let result;
        (self.remain, result) = OpRaw::parse(self.remain).unwrap();
        result.map(|x| Op::from_raw(x, self.fdt.strings_block()))
    }
}
