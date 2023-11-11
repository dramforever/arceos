use core::ffi::CStr;
use nom::{bytes::complete::*, combinator::*, error::*, number::complete::*, sequence::*, *};

const FDT_BEGIN_NODE: u32 = 0x1;
const FDT_END_NODE: u32 = 0x2;
const FDT_PROP: u32 = 0x3;
const FDT_NOP: u32 = 0x4;
const FDT_END: u32 = 0x9;

#[cfg_attr(test, derive(Debug))]
pub(crate) enum OpRaw<'a> {
    BeginNode { name: &'a CStr },
    EndNode,
    Prop { name_offset: usize, value: &'a [u8] },
}

fn read_c_string(input: &[u8]) -> Option<(&CStr, &[u8])> {
    let position = input.iter().position(|&x| x == 0)?;
    let (string, res) = input.split_at(position + 1);
    Some((CStr::from_bytes_with_nul(string).unwrap(), res))
}

fn padding_len(len: usize) -> usize {
    let rounded = (len + 3) / 4 * 4;
    rounded - len
}

fn c_string(input: &[u8]) -> IResult<&[u8], &CStr> {
    let (result, input) = read_c_string(input).ok_or(Err::Incomplete(Needed::Unknown))?;
    Ok((input, result))
}

impl<'a> OpRaw<'a> {
    pub(crate) fn parse(mut input: &'a [u8]) -> IResult<&'a [u8], Option<Self>> {
        loop {
            let op;
            (input, op) = be_u32(input)?;
            let res = match op {
                FDT_END => {
                    (input, _) = eof(input)?;
                    return Ok((input, None));
                }
                FDT_NOP => continue,
                FDT_BEGIN_NODE => {
                    let name;
                    (input, name) = c_string(input)?;
                    (input, _) = take(padding_len(name.to_bytes_with_nul().len()))(input)?;
                    Self::BeginNode { name }
                }
                FDT_END_NODE => Self::EndNode,
                FDT_PROP => {
                    let (len, name_offset);
                    (input, (len, name_offset)) = tuple((be_u32, be_u32))(input)?;
                    let len = len as usize;
                    let value;
                    (input, value) = take(len)(input)?;
                    (input, _) = take(padding_len(len))(input)?;

                    Self::Prop {
                        name_offset: name_offset as usize,
                        value,
                    }
                }
                _ => {
                    return Err(Err::Error(Error {
                        input,
                        code: ErrorKind::Tag,
                    }))
                }
            };
            break Ok((input, Some(res)));
        }
    }
}

pub(crate) enum Op<'a> {
    BeginNode { name: &'a CStr },
    EndNode,
    Prop { name: &'a CStr, value: &'a [u8] },
}

impl<'a> Op<'a> {
    pub(crate) fn from_raw(raw: OpRaw<'a>, strings: &'a [u8]) -> Op<'a> {
        match raw {
            OpRaw::BeginNode { name } => Op::BeginNode { name },
            OpRaw::EndNode => Op::EndNode,
            OpRaw::Prop { name_offset, value } => Op::Prop {
                name: match read_c_string(&strings[name_offset..]) {
                    Some(x) => x.0,
                    None => panic!("{:?}", &strings[name_offset..]),
                },
                value,
            },
        }
    }
}
