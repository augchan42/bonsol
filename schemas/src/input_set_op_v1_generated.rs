// automatically generated by the FlatBuffers compiler, do not modify


// @generated

use crate::input_type_generated::*;
use core::mem;
use core::cmp::Ordering;

extern crate flatbuffers;
use self::flatbuffers::{EndianScalar, Follow};

#[deprecated(since = "2.0.0", note = "Use associated constants instead. This will no longer be generated in 2021.")]
pub const ENUM_MIN_INPUT_SET_OP: u8 = 0;
#[deprecated(since = "2.0.0", note = "Use associated constants instead. This will no longer be generated in 2021.")]
pub const ENUM_MAX_INPUT_SET_OP: u8 = 2;
#[deprecated(since = "2.0.0", note = "Use associated constants instead. This will no longer be generated in 2021.")]
#[allow(non_camel_case_types)]
pub const ENUM_VALUES_INPUT_SET_OP: [InputSetOp; 3] = [
  InputSetOp::Create,
  InputSetOp::Update,
  InputSetOp::Delete,
];

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[repr(transparent)]
pub struct InputSetOp(pub u8);
#[allow(non_upper_case_globals)]
impl InputSetOp {
  pub const Create: Self = Self(0);
  pub const Update: Self = Self(1);
  pub const Delete: Self = Self(2);

  pub const ENUM_MIN: u8 = 0;
  pub const ENUM_MAX: u8 = 2;
  pub const ENUM_VALUES: &'static [Self] = &[
    Self::Create,
    Self::Update,
    Self::Delete,
  ];
  /// Returns the variant's name or "" if unknown.
  pub fn variant_name(self) -> Option<&'static str> {
    match self {
      Self::Create => Some("Create"),
      Self::Update => Some("Update"),
      Self::Delete => Some("Delete"),
      _ => None,
    }
  }
}
impl core::fmt::Debug for InputSetOp {
  fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
    if let Some(name) = self.variant_name() {
      f.write_str(name)
    } else {
      f.write_fmt(format_args!("<UNKNOWN {:?}>", self.0))
    }
  }
}
impl<'a> flatbuffers::Follow<'a> for InputSetOp {
  type Inner = Self;
  #[inline]
  unsafe fn follow(buf: &'a [u8], loc: usize) -> Self::Inner {
    let b = flatbuffers::read_scalar_at::<u8>(buf, loc);
    Self(b)
  }
}

impl flatbuffers::Push for InputSetOp {
    type Output = InputSetOp;
    #[inline]
    unsafe fn push(&self, dst: &mut [u8], _written_len: usize) {
        flatbuffers::emplace_scalar::<u8>(dst, self.0);
    }
}

impl flatbuffers::EndianScalar for InputSetOp {
  type Scalar = u8;
  #[inline]
  fn to_little_endian(self) -> u8 {
    self.0.to_le()
  }
  #[inline]
  #[allow(clippy::wrong_self_convention)]
  fn from_little_endian(v: u8) -> Self {
    let b = u8::from_le(v);
    Self(b)
  }
}

impl<'a> flatbuffers::Verifiable for InputSetOp {
  #[inline]
  fn run_verifier(
    v: &mut flatbuffers::Verifier, pos: usize
  ) -> Result<(), flatbuffers::InvalidFlatbuffer> {
    use self::flatbuffers::Verifiable;
    u8::run_verifier(v, pos)
  }
}

impl flatbuffers::SimpleToVerifyInSlice for InputSetOp {}
pub enum InputSetOpV1Offset {}
#[derive(Copy, Clone, PartialEq)]

pub struct InputSetOpV1<'a> {
  pub _tab: flatbuffers::Table<'a>,
}

impl<'a> flatbuffers::Follow<'a> for InputSetOpV1<'a> {
  type Inner = InputSetOpV1<'a>;
  #[inline]
  unsafe fn follow(buf: &'a [u8], loc: usize) -> Self::Inner {
    Self { _tab: flatbuffers::Table::new(buf, loc) }
  }
}

impl<'a> InputSetOpV1<'a> {
  pub const VT_ID: flatbuffers::VOffsetT = 4;
  pub const VT_OP: flatbuffers::VOffsetT = 6;
  pub const VT_INPUTS: flatbuffers::VOffsetT = 8;

  #[inline]
  pub unsafe fn init_from_table(table: flatbuffers::Table<'a>) -> Self {
    InputSetOpV1 { _tab: table }
  }
  #[allow(unused_mut)]
  pub fn create<'bldr: 'args, 'args: 'mut_bldr, 'mut_bldr, A: flatbuffers::Allocator + 'bldr>(
    _fbb: &'mut_bldr mut flatbuffers::FlatBufferBuilder<'bldr, A>,
    args: &'args InputSetOpV1Args<'args>
  ) -> flatbuffers::WIPOffset<InputSetOpV1<'bldr>> {
    let mut builder = InputSetOpV1Builder::new(_fbb);
    if let Some(x) = args.inputs { builder.add_inputs(x); }
    if let Some(x) = args.id { builder.add_id(x); }
    builder.add_op(args.op);
    builder.finish()
  }

  pub fn unpack(&self) -> InputSetOpV1T {
    let id = self.id().map(|x| {
      x.to_string()
    });
    let op = self.op();
    let inputs = self.inputs().map(|x| {
      x.iter().map(|t| t.unpack()).collect()
    });
    InputSetOpV1T {
      id,
      op,
      inputs,
    }
  }

  #[inline]
  pub fn id(&self) -> Option<&'a str> {
    // Safety:
    // Created from valid Table for this object
    // which contains a valid value in this slot
    unsafe { self._tab.get::<flatbuffers::ForwardsUOffset<&str>>(InputSetOpV1::VT_ID, None)}
  }
  #[inline]
  pub fn op(&self) -> InputSetOp {
    // Safety:
    // Created from valid Table for this object
    // which contains a valid value in this slot
    unsafe { self._tab.get::<InputSetOp>(InputSetOpV1::VT_OP, Some(InputSetOp::Create)).unwrap()}
  }
  #[inline]
  pub fn inputs(&self) -> Option<flatbuffers::Vector<'a, flatbuffers::ForwardsUOffset<Input<'a>>>> {
    // Safety:
    // Created from valid Table for this object
    // which contains a valid value in this slot
    unsafe { self._tab.get::<flatbuffers::ForwardsUOffset<flatbuffers::Vector<'a, flatbuffers::ForwardsUOffset<Input>>>>(InputSetOpV1::VT_INPUTS, None)}
  }
}

impl flatbuffers::Verifiable for InputSetOpV1<'_> {
  #[inline]
  fn run_verifier(
    v: &mut flatbuffers::Verifier, pos: usize
  ) -> Result<(), flatbuffers::InvalidFlatbuffer> {
    use self::flatbuffers::Verifiable;
    v.visit_table(pos)?
     .visit_field::<flatbuffers::ForwardsUOffset<&str>>("id", Self::VT_ID, false)?
     .visit_field::<InputSetOp>("op", Self::VT_OP, false)?
     .visit_field::<flatbuffers::ForwardsUOffset<flatbuffers::Vector<'_, flatbuffers::ForwardsUOffset<Input>>>>("inputs", Self::VT_INPUTS, false)?
     .finish();
    Ok(())
  }
}
pub struct InputSetOpV1Args<'a> {
    pub id: Option<flatbuffers::WIPOffset<&'a str>>,
    pub op: InputSetOp,
    pub inputs: Option<flatbuffers::WIPOffset<flatbuffers::Vector<'a, flatbuffers::ForwardsUOffset<Input<'a>>>>>,
}
impl<'a> Default for InputSetOpV1Args<'a> {
  #[inline]
  fn default() -> Self {
    InputSetOpV1Args {
      id: None,
      op: InputSetOp::Create,
      inputs: None,
    }
  }
}

pub struct InputSetOpV1Builder<'a: 'b, 'b, A: flatbuffers::Allocator + 'a> {
  fbb_: &'b mut flatbuffers::FlatBufferBuilder<'a, A>,
  start_: flatbuffers::WIPOffset<flatbuffers::TableUnfinishedWIPOffset>,
}
impl<'a: 'b, 'b, A: flatbuffers::Allocator + 'a> InputSetOpV1Builder<'a, 'b, A> {
  #[inline]
  pub fn add_id(&mut self, id: flatbuffers::WIPOffset<&'b  str>) {
    self.fbb_.push_slot_always::<flatbuffers::WIPOffset<_>>(InputSetOpV1::VT_ID, id);
  }
  #[inline]
  pub fn add_op(&mut self, op: InputSetOp) {
    self.fbb_.push_slot::<InputSetOp>(InputSetOpV1::VT_OP, op, InputSetOp::Create);
  }
  #[inline]
  pub fn add_inputs(&mut self, inputs: flatbuffers::WIPOffset<flatbuffers::Vector<'b , flatbuffers::ForwardsUOffset<Input<'b >>>>) {
    self.fbb_.push_slot_always::<flatbuffers::WIPOffset<_>>(InputSetOpV1::VT_INPUTS, inputs);
  }
  #[inline]
  pub fn new(_fbb: &'b mut flatbuffers::FlatBufferBuilder<'a, A>) -> InputSetOpV1Builder<'a, 'b, A> {
    let start = _fbb.start_table();
    InputSetOpV1Builder {
      fbb_: _fbb,
      start_: start,
    }
  }
  #[inline]
  pub fn finish(self) -> flatbuffers::WIPOffset<InputSetOpV1<'a>> {
    let o = self.fbb_.end_table(self.start_);
    flatbuffers::WIPOffset::new(o.value())
  }
}

impl core::fmt::Debug for InputSetOpV1<'_> {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    let mut ds = f.debug_struct("InputSetOpV1");
      ds.field("id", &self.id());
      ds.field("op", &self.op());
      ds.field("inputs", &self.inputs());
      ds.finish()
  }
}
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub struct InputSetOpV1T {
  pub id: Option<String>,
  pub op: InputSetOp,
  pub inputs: Option<Vec<InputT>>,
}
impl Default for InputSetOpV1T {
  fn default() -> Self {
    Self {
      id: None,
      op: InputSetOp::Create,
      inputs: None,
    }
  }
}
impl InputSetOpV1T {
  pub fn pack<'b, A: flatbuffers::Allocator + 'b>(
    &self,
    _fbb: &mut flatbuffers::FlatBufferBuilder<'b, A>
  ) -> flatbuffers::WIPOffset<InputSetOpV1<'b>> {
    let id = self.id.as_ref().map(|x|{
      _fbb.create_string(x)
    });
    let op = self.op;
    let inputs = self.inputs.as_ref().map(|x|{
      let w: Vec<_> = x.iter().map(|t| t.pack(_fbb)).collect();_fbb.create_vector(&w)
    });
    InputSetOpV1::create(_fbb, &InputSetOpV1Args{
      id,
      op,
      inputs,
    })
  }
}
#[inline]
/// Verifies that a buffer of bytes contains a `InputSetOpV1`
/// and returns it.
/// Note that verification is still experimental and may not
/// catch every error, or be maximally performant. For the
/// previous, unchecked, behavior use
/// `root_as_input_set_op_v1_unchecked`.
pub fn root_as_input_set_op_v1(buf: &[u8]) -> Result<InputSetOpV1, flatbuffers::InvalidFlatbuffer> {
  flatbuffers::root::<InputSetOpV1>(buf)
}
#[inline]
/// Verifies that a buffer of bytes contains a size prefixed
/// `InputSetOpV1` and returns it.
/// Note that verification is still experimental and may not
/// catch every error, or be maximally performant. For the
/// previous, unchecked, behavior use
/// `size_prefixed_root_as_input_set_op_v1_unchecked`.
pub fn size_prefixed_root_as_input_set_op_v1(buf: &[u8]) -> Result<InputSetOpV1, flatbuffers::InvalidFlatbuffer> {
  flatbuffers::size_prefixed_root::<InputSetOpV1>(buf)
}
#[inline]
/// Verifies, with the given options, that a buffer of bytes
/// contains a `InputSetOpV1` and returns it.
/// Note that verification is still experimental and may not
/// catch every error, or be maximally performant. For the
/// previous, unchecked, behavior use
/// `root_as_input_set_op_v1_unchecked`.
pub fn root_as_input_set_op_v1_with_opts<'b, 'o>(
  opts: &'o flatbuffers::VerifierOptions,
  buf: &'b [u8],
) -> Result<InputSetOpV1<'b>, flatbuffers::InvalidFlatbuffer> {
  flatbuffers::root_with_opts::<InputSetOpV1<'b>>(opts, buf)
}
#[inline]
/// Verifies, with the given verifier options, that a buffer of
/// bytes contains a size prefixed `InputSetOpV1` and returns
/// it. Note that verification is still experimental and may not
/// catch every error, or be maximally performant. For the
/// previous, unchecked, behavior use
/// `root_as_input_set_op_v1_unchecked`.
pub fn size_prefixed_root_as_input_set_op_v1_with_opts<'b, 'o>(
  opts: &'o flatbuffers::VerifierOptions,
  buf: &'b [u8],
) -> Result<InputSetOpV1<'b>, flatbuffers::InvalidFlatbuffer> {
  flatbuffers::size_prefixed_root_with_opts::<InputSetOpV1<'b>>(opts, buf)
}
#[inline]
/// Assumes, without verification, that a buffer of bytes contains a InputSetOpV1 and returns it.
/// # Safety
/// Callers must trust the given bytes do indeed contain a valid `InputSetOpV1`.
pub unsafe fn root_as_input_set_op_v1_unchecked(buf: &[u8]) -> InputSetOpV1 {
  flatbuffers::root_unchecked::<InputSetOpV1>(buf)
}
#[inline]
/// Assumes, without verification, that a buffer of bytes contains a size prefixed InputSetOpV1 and returns it.
/// # Safety
/// Callers must trust the given bytes do indeed contain a valid size prefixed `InputSetOpV1`.
pub unsafe fn size_prefixed_root_as_input_set_op_v1_unchecked(buf: &[u8]) -> InputSetOpV1 {
  flatbuffers::size_prefixed_root_unchecked::<InputSetOpV1>(buf)
}
#[inline]
pub fn finish_input_set_op_v1_buffer<'a, 'b, A: flatbuffers::Allocator + 'a>(
    fbb: &'b mut flatbuffers::FlatBufferBuilder<'a, A>,
    root: flatbuffers::WIPOffset<InputSetOpV1<'a>>) {
  fbb.finish(root, None);
}

#[inline]
pub fn finish_size_prefixed_input_set_op_v1_buffer<'a, 'b, A: flatbuffers::Allocator + 'a>(fbb: &'b mut flatbuffers::FlatBufferBuilder<'a, A>, root: flatbuffers::WIPOffset<InputSetOpV1<'a>>) {
  fbb.finish_size_prefixed(root, None);
}
