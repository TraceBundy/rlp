// Copyright 2020 Parity Technologies
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#[cfg(not(feature = "std"))]
use alloc::{borrow::ToOwned, boxed::Box, string::String, vec::Vec};
use core::iter::{empty, once};
use core::{mem, str};

use crate::error::DecoderError;
use crate::rlpin::Rlp;
use crate::stream::RlpStream;
use crate::traits::{Decodable, Encodable};

pub fn decode_usize(bytes: &[u8]) -> Result<usize, DecoderError> {
	match bytes.len() {
		l if l <= mem::size_of::<usize>() => {
			if bytes[0] == 0 {
				return Err(DecoderError::RlpInvalidIndirection);
			}
			let mut res = 0usize;
			for (i, byte) in bytes.iter().enumerate().take(l) {
				let shift = (l - 1 - i) * 8;
				res += (*byte as usize) << shift;
			}
			Ok(res)
		}
		_ => Err(DecoderError::RlpIsTooBig),
	}
}

impl<T: Encodable + ?Sized> Encodable for Box<T> {
	fn rlp_append(&self, s: &mut RlpStream) {
		Encodable::rlp_append(&**self, s)
	}
}

impl<T: Decodable> Decodable for Box<T> {
	fn decode(rlp: &Rlp) -> Result<Self, DecoderError> {
		T::decode(rlp).map(Box::new)
	}
}

impl Encodable for bool {
	fn rlp_append(&self, s: &mut RlpStream) {
		s.encoder().encode_iter(once(if *self { 1u8 } else { 0 }));
	}
}

impl Decodable for bool {
	fn decode(rlp: &Rlp) -> Result<Self, DecoderError> {
		rlp.decoder().decode_value(|bytes| match bytes.len() {
			0 => Ok(false),
			1 => Ok(bytes[0] != 0),
			_ => Err(DecoderError::RlpIsTooBig),
		})
	}
}

impl<'a> Encodable for &'a [u8] {
	fn rlp_append(&self, s: &mut RlpStream) {
		s.encoder().encode_value(self);
	}
}

impl Encodable for Vec<u8> {
	fn rlp_append(&self, s: &mut RlpStream) {
		s.encoder().encode_value(self);
	}
}

impl Decodable for Vec<u8> {
	fn decode(rlp: &Rlp) -> Result<Self, DecoderError> {
		rlp.decoder().decode_value(|bytes| Ok(bytes.to_vec()))
	}
}

impl<T> Encodable for Option<T>
where
	T: Encodable,
{
	fn rlp_append(&self, s: &mut RlpStream) {
		match *self {
			None => {
				s.begin_list(0);
			}
			Some(ref value) => {
				s.begin_list(1);
				s.append(value);
			}
		}
	}
}

impl<T> Decodable for Option<T>
where
	T: Decodable,
{
	fn decode(rlp: &Rlp) -> Result<Self, DecoderError> {
		let items = rlp.item_count()?;
		match items {
			1 => rlp.val_at(0).map(Some),
			0 => Ok(None),
			_ => Err(DecoderError::RlpIncorrectListLen),
		}
	}
}

impl Encodable for u8 {
	fn rlp_append(&self, s: &mut RlpStream) {
		if *self != 0 {
			s.encoder().encode_iter(once(*self));
		} else {
			s.encoder().encode_iter(empty());
		}
	}
}

impl Decodable for u8 {
	fn decode(rlp: &Rlp) -> Result<Self, DecoderError> {
		rlp.decoder().decode_value(|bytes| match bytes.len() {
			1 if bytes[0] != 0 => Ok(bytes[0]),
			0 => Ok(0),
			1 => Err(DecoderError::RlpInvalidIndirection),
			_ => Err(DecoderError::RlpIsTooBig),
		})
	}
}

macro_rules! impl_encodable_for_u {
	($name: ident) => {
		impl Encodable for $name {
			fn rlp_append(&self, s: &mut RlpStream) {
				let leading_empty_bytes = self.leading_zeros() as usize / 8;
				let buffer = self.to_be_bytes();
				s.encoder().encode_value(&buffer[leading_empty_bytes..]);
			}
		}
	};
}

macro_rules! impl_decodable_for_u {
	($name: ident) => {
		impl Decodable for $name {
			fn decode(rlp: &Rlp) -> Result<Self, DecoderError> {
				rlp.decoder().decode_value(|bytes| match bytes.len() {
					0 | 1 => u8::decode(rlp).map(|v| v as $name),
					l if l <= mem::size_of::<$name>() => {
						if bytes[0] == 0 {
							return Err(DecoderError::RlpInvalidIndirection);
						}
						let mut res = 0 as $name;
						for (i, byte) in bytes.iter().enumerate().take(l) {
							let shift = (l - 1 - i) * 8;
							res += (*byte as $name) << shift;
						}
						Ok(res)
					}
					_ => Err(DecoderError::RlpIsTooBig),
				})
			}
		}
	};
}

impl_encodable_for_u!(u16);
impl_encodable_for_u!(u32);
impl_encodable_for_u!(u64);
impl_encodable_for_u!(u128);

impl_decodable_for_u!(u16);
impl_decodable_for_u!(u32);
impl_decodable_for_u!(u64);
impl_decodable_for_u!(u128);

impl Encodable for usize {
	fn rlp_append(&self, s: &mut RlpStream) {
		(*self as u64).rlp_append(s);
	}
}

impl Decodable for usize {
	fn decode(rlp: &Rlp) -> Result<Self, DecoderError> {
		u64::decode(rlp).map(|value| value as usize)
	}
}

impl<'a> Encodable for &'a str {
	fn rlp_append(&self, s: &mut RlpStream) {
		s.encoder().encode_value(self.as_bytes());
	}
}

impl Encodable for String {
	fn rlp_append(&self, s: &mut RlpStream) {
		s.encoder().encode_value(self.as_bytes());
	}
}

impl Decodable for String {
	fn decode(rlp: &Rlp) -> Result<Self, DecoderError> {
		rlp.decoder().decode_value(|bytes| {
			match str::from_utf8(bytes) {
				Ok(s) => Ok(s.to_owned()),
				// consider better error type here
				Err(_err) => Err(DecoderError::RlpExpectedToBeData),
			}
		})
	}
}

macro_rules! impl_encodable_for_i {
	($name: ident) => {
		impl Encodable for $name {
			fn rlp_append(&self, s: &mut RlpStream) {
				let i = *self as i128;
				let zigzag = ((i << 1) ^ (i >> 127)) as u128;
				let leading_empty_bytes = zigzag.leading_zeros() as usize / 8;
				let buffer = zigzag.to_be_bytes();
				s.encoder().encode_value(&buffer[leading_empty_bytes..]);
			}
		}
	};
}

macro_rules! impl_decodable_for_i {
	($name: ident) => {
		impl Decodable for $name {
			fn decode(rlp: &Rlp) -> Result<Self, DecoderError> {
				match u128::decode(rlp) {
					Ok(res) => {
						let recover = ((res >> 1) ^ (-((res & 1) as i128)) as u128) as $name;
						Ok(recover)
					}
					Err(err) => Err(err),
				}
			}
		}
	};
}

impl_encodable_for_i!(i8);
impl_encodable_for_i!(i16);
impl_encodable_for_i!(i32);
impl_encodable_for_i!(i64);
impl_encodable_for_i!(i128);

impl_decodable_for_i!(i8);
impl_decodable_for_i!(i16);
impl_decodable_for_i!(i32);
impl_decodable_for_i!(i64);
impl_decodable_for_i!(i128);

macro_rules! impl_encodable_for_f {
	($name: ident, $value : ident) => {
		impl Encodable for $name {
			fn rlp_append(&self, s: &mut RlpStream) {
				let num = $value::from_be_bytes(self.to_bits().to_be_bytes());
				num.rlp_append(s);
			}
		}
	};
}

macro_rules! impl_decodable_for_f {
	($name: ident, $value : ident) => {
		impl Decodable for $name {
			fn decode(rlp: &Rlp) -> Result<Self, DecoderError> {
				match $value::decode(rlp) {
					Ok(num) => Ok($name::from_bits(num)),
					Err(err) => Err(err),
				}
			}
		}
	};
}
impl_encodable_for_f!(f32, u32);
impl_decodable_for_f!(f32, u32);
impl_encodable_for_f!(f64, u64);
impl_decodable_for_f!(f64, u64);



#[macro_export]
macro_rules! impl_array_rlp {
	($size: expr) => {
		impl Encodable for [u8;$size] {
			fn rlp_append(&self, s: &mut RlpStream) {
				s.encoder().encode_value(self.as_ref());
			}
		}

		impl Decodable for [u8;$size] {
			fn decode(rlp: &Rlp) -> Result<Self, DecoderError> {
				rlp.decoder().decode_value(|bytes| match bytes.len().cmp(&$size) {
					std::cmp::Ordering::Less => Err(DecoderError::RlpIsTooShort),
					std::cmp::Ordering::Greater => Err(DecoderError::RlpIsTooBig),
					std::cmp::Ordering::Equal => {
						let mut t = [0u8; $size];
						t.copy_from_slice(bytes);
						Ok(t)
					}
				})
			}
		}
	};
}

impl_array_rlp!(4);
impl_array_rlp!(8);
impl_array_rlp!(16);
impl_array_rlp!(32);
impl_array_rlp!(64);
impl_array_rlp!(128);