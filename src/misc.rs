use std::fmt;

use anyhow::bail;

#[derive(Clone, Copy, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub struct u24([u8; 3]);

impl u24 {
	pub fn as_u32(&self) -> u32 {
		(self.0[2] as u32) << 16 | (self.0[1] as u32) << 8 | self.0[0] as u32
	}

	pub fn as_usize(&self) -> usize {
		(self.0[2] as usize) << 16 | (self.0[1] as usize) << 8 | self.0[0] as usize
	}

	pub fn to_bytes(&self) -> [u8; 3] {
		self.0
	}

	pub const fn max() -> u32 {
		16777215 // 1 << 24 - 1
	}
}

impl From<u24> for u32 {
	fn from(value: u24) -> Self {
		value.as_u32()
	}
}

impl From<u24> for usize {
	fn from(value: u24) -> Self {
		value.as_usize()
	}
}

impl From<u8> for u24 {
	fn from(value: u8) -> Self {
		Self([value, 0, 0])
	}
}

impl From<u16> for u24 {
	fn from(value: u16) -> Self {
		let value = value.to_le_bytes();
		Self([value[0], value[1], 0])
	}
}

impl TryFrom<u32> for u24 {
	type Error = anyhow::Error;

	fn try_from(value: u32) -> std::result::Result<Self, Self::Error> {
		if value < 1 << 24 {
			let value = value.to_le_bytes();
			Ok(Self([value[0], value[1], value[2]]))
		} else {
			bail!("Overflow! {} >= u24 max ({})", value, 1 << 24)
		}
	}
}

impl TryFrom<usize> for u24 {
	type Error = anyhow::Error;

	fn try_from(value: usize) -> std::result::Result<Self, Self::Error> {
		if value < 1 << 24 {
			let value = value.to_le_bytes();
			Ok(Self([value[0], value[1], value[2]]))
		} else {
			bail!("Overflow! {} >= u24 max ({})", value, 1 << 24)
		}
	}
}

impl TryFrom<&[u8]> for u24 {
	type Error = anyhow::Error;

	fn try_from(value: &[u8]) -> std::result::Result<Self, Self::Error> {
		if value.len() == 3 {
			Ok(Self([value[0], value[1], value[2]]))
		} else {
			bail!("unable to convert a {}-byte {:X?} into a u24", value.len(), value)
		}
	}
}

impl fmt::Debug for u24 {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_tuple("u24").field(&self.as_u32()).finish()
	}
}

impl fmt::Display for u24 {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{}", self.as_u32())
	}
}
