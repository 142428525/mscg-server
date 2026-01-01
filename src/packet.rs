use std::{fmt, time};

use anyhow::{Result, bail, ensure};
use prost::Message;

pub(crate) mod msg {
	use anyhow::{Result, bail, ensure};
	use prost::Message;

	use crate::misc::u24;

	include!(concat!(env!("OUT_DIR"), "/api_pf.rs"));

	pub const MAGIC: u32 = 0x0d000721;

	/// NOTE: Remember to increase this when any of the "oneof" fields or the field numbers changed.
	///
	/// No use in dev version.
	pub const VERSION: u8 = 1;

	#[repr(u8)]
	#[derive(Clone, Copy)]
	pub enum MsgType {
		UNSPECIFIED,
		REQUEST,
		RESPONSE,
		ANOUNCEMENT,
		HEARTBEAT,
	}

	#[repr(C)]
	#[derive(Debug, PartialEq, Eq)]
	pub struct MsgHead {
		pub magic: u32,
		pub ver: u8,
		pub session_id: u24,
		pub ty: u8,
		pub len: u24,
		pub crc32: u32,
	}

	impl MsgHead {
		pub fn build(ty: u8, session_id: u24, len: u24, crc32: u32) -> Self {
			Self {
				magic: MAGIC,
				ver: VERSION,
				session_id,
				ty,
				len,
				crc32,
			}
		}

		pub const fn len() -> usize {
			16
		}

		pub fn encode(&self, buf: &mut impl bytes::BufMut) -> Result<()> {
			let required = Self::len();
			let remaining = buf.remaining_mut();
			if required > remaining {
				bail!("Too short buf len to encode: {remaining} (expected {required})");
			}

			buf.put_u32(self.magic);
			buf.put_u8(self.ver);
			buf.put(&self.session_id.to_bytes()[..]);
			buf.put_u8(self.ty);
			buf.put(&self.len.to_bytes()[..]);
			buf.put_u32(self.crc32);
			Ok(())
		}

		pub fn encode_to_vec(&self) -> Vec<u8> {
			let mut tmp = bytes::BytesMut::with_capacity(Self::len());
			self.encode(&mut tmp).unwrap();
			tmp.to_vec()
		}

		pub fn decode(mut buf: impl bytes::Buf) -> Result<Self> {
			Self::check_buf(&buf)?;
			Ok(Self {
				magic: buf.get_u32(),
				ver: buf.get_u8(),
				session_id: buf.copy_to_bytes(3)[..].try_into().unwrap(),
				ty: buf.get_u8(),
				len: buf.copy_to_bytes(3)[..].try_into().unwrap(),
				crc32: buf.get_u32(),
			})
		}

		fn check_buf(buf: &impl bytes::Buf) -> Result<()> {
			let expected = Self::len();
			let remaining = buf.remaining();
			if expected > remaining {
				bail!("Too short buf len to decode: {remaining} (expected {expected})");
			}

			let peeking = &buf.chunk()[..expected];
			ensure!(
				peeking[..4] == MAGIC.to_be_bytes(),
				"Invalid magic number, shown as bytes: {:X?} (expected {:X?})",
				&peeking[..4],
				MAGIC.to_be_bytes()
			);
			ensure!(
				peeking[4] == VERSION,
				"Unmatched msg version: {} (expected {})",
				peeking[4],
				VERSION
			);

			Ok(())
		}
	}

	#[derive(Debug, PartialEq, Eq)]
	pub struct Msg(pub MsgHead, pub MsgBody);

	impl Msg {
		pub fn build<T>(ty: MsgType, session_id: u24, body: T) -> Result<Self>
		where
			T: Into<msg_body::Body> + prost::Message,
		{
			let msg_body = MsgBody {
				body: Some(body.into()),
			};

			let len = msg_body.encoded_len();
			ensure!(
				len <= u24::max() as usize,
				"Too long message body (> 16MiB). Why doing so???"
			);

			Ok(Self(
				MsgHead::build(
					ty as u8,
					session_id,
					len.try_into().unwrap(),
					crc32fast::hash(&msg_body.encode_to_vec()),
				),
				msg_body,
			))
		}

		pub fn encode(&self, buf: &mut impl bytes::BufMut) -> Result<()> {
			let required = MsgHead::len() + self.0.len.as_usize();
			let remaining = buf.remaining_mut();
			if required > remaining {
				bail!("Too short buf len to encode: {remaining} (expected {required})");
			}

			self.0.encode(buf).unwrap();
			self.1.encode(buf).unwrap();

			Ok(())
		}

		pub fn encode_to_vec(&self) -> Vec<u8> {
			let mut tmp = Vec::with_capacity(MsgHead::len() + self.0.len.as_usize());
			self.0.encode(&mut tmp).unwrap();
			self.1.encode(&mut tmp).unwrap();

			tmp
		}
	}
}

use msg::*;

use crate::misc::u24;

/// Returns Ok(None) if no enough data to decode a MsgHead.
pub fn try_parse_head(mut buf: impl bytes::Buf + fmt::Debug) -> Result<Option<MsgHead>> {
	let len = MsgHead::len();
	if buf.remaining() < len {
		log::warn!(
			"parse_head: buf remaining only: {} (expected >= {})",
			buf.remaining(),
			len
		);
		return Ok(None);
	}

	let tmp = buf.chunk().get(..len);
	let tmp = match tmp {
		Some(data) => data.to_owned(),
		_ => bail!("Unexpectedly can't get enough data from buf to parse a MsgHead!"),
	};

	// if tmp is full of 0, for its magic number is [0, 0, 0, 0]
	ensure!(
		tmp[..4] != [0, 0, 0, 0],
		"Haven't received data yet, buf still full of 0! This error should be ignored."
	);

	log::debug!("recv head: {:X?}", tmp);
	buf.advance(len);

	match MsgHead::decode(&tmp[..]) {
		Ok(x) => Ok(Some(x)),
		Err(e) => Err(e.context(format!("Can't decode a MsgHead from: {:X?}", tmp))),
	}
}

pub fn try_parse_body(head: &MsgHead, mut buf: impl bytes::Buf + fmt::Debug) -> Result<Option<MsgBody>> {
	let len = head.len.into();
	if buf.remaining() < len {
		log::warn!(
			"parse_body: buf remaining only: {} (expected >= {})",
			buf.remaining(),
			len
		);
		return Ok(None);
	}

	let tmp = buf.chunk().get(..len);
	let tmp = match tmp {
		Some(data) => data.to_owned(),
		_ => bail!("Unexpectedly can't get enough data from buf to parse a MsgBody!"),
	};

	// if tmp is full of 0, for its protobuf message head is 0 (expected >= 1)
	ensure!(
		tmp[0] != 0,
		"Haven't received data yet, buf still full of 0! This error should be ignored."
	);

	log::debug!("recv body: {:X?}", tmp);
	buf.advance(len);

	match MsgBody::decode(&tmp[..]) {
		Ok(y @ MsgBody { body: Some(x) }) => {
			let mut tmp_buf = bytes::BytesMut::new();
			x.encode(&mut tmp_buf);
			let tmp_crc32 = crc32fast::hash(&tmp_buf);
			if tmp_crc32 == head.crc32 {
				Ok(Some(y))
			} else {
				bail!("CRC32 verification fails: {:X} (expected {:X})", tmp_crc32, head.crc32);
			}
		}
		Err(e) => bail!("{:?}\nCan't decode a MsgBody from: {:X?}", e, tmp), // why doesn't DecodeError derive from Error???
		Ok(_) => bail!("WTF a MsgBody of None???"),
	}
}

pub fn build_heartbeat_msg(session_id: u24) -> Msg {
	Msg::build(
		MsgType::HEARTBEAT,
		session_id,
		Heartbeat {
			ts: Some(time::SystemTime::now().into()),
		},
	)
	.unwrap() // assert no fault
}

#[cfg(test)]
mod tests {
	use std::io;

	use anyhow::Ok;

	use super::*;

	#[test]
	fn parse_separately() -> Result<()> {
		let msg = build_heartbeat_msg(11451_u16.into());
		println!("msg: {:?}", msg);

		let mut buf = bytes::BytesMut::with_capacity(64);
		//buf.resize(64, 0);

		msg.0.encode(&mut buf)?;
		println!("buf0: {:X?}", &buf[..]);

		let h = MsgHead::decode(&mut buf)?;
		println!("0: {:?}", h);
		println!("buf: {:X?}", buf);

		msg.1.encode(&mut buf)?;
		println!("buf1: {:X?}", &buf[..]);

		let b = MsgBody::decode(&mut buf)?;
		println!("1: {:?}", b);
		println!("buf: {:X?}", buf);

		assert_eq!(msg, Msg(h, b));

		Ok(())
	}

	#[test]
	fn parse_as_a_whole() -> Result<()> {
		let msg = build_heartbeat_msg(11451_u16.into());
		println!("msg: {:?}", msg);

		let mut buf = bytes::BytesMut::with_capacity(128);

		msg.0.encode(&mut buf)?;
		println!("buf0: {:X?}", &buf[..]);

		msg.1.encode(&mut buf)?;
		println!("buf1: {:X?}", &buf[..]);

		let h = try_parse_head(&mut buf)?.unwrap();
		println!("0: {:?}", h);
		println!("buf: {:X?}", buf);

		let b = try_parse_body(&h, &mut buf)?.unwrap();
		println!("1: {:?}", b);
		println!("buf: {:X?}", buf);

		assert_eq!(msg, Msg(h, b));

		Ok(())
	}

	#[test]
	#[ignore = "nope"]
	fn length_delimited() -> Result<()> {
		let msg = build_heartbeat_msg(0_u8.into());
		println!("msg: {:?}", msg);

		let mut buf = bytes::BytesMut::with_capacity(64);
		//buf.resize(64, 0);

		msg.0.encode(&mut buf)?;
		println!("buf0: {:X?}", &buf[..]);

		//let h = MsgHead::decode_length_delimited(&mut buf)?;
		//println!("0: {:?}", h);
		println!("buf: {:X?}", buf);

		msg.1.encode_length_delimited(&mut buf)?;
		println!("buf1: {:X?}", &buf[..]);

		let b = MsgBody::decode(&mut buf)?;
		println!("1: {:?}", b);
		println!("buf: {:X?}", buf);

		//assert_eq!(msg, (h, b));

		Ok(())
	}

	#[test]
	#[ignore = "nope"]
	fn decode_wtf() -> Result<()> {
		// let head: [u8; 20] = [
		// 	0xD, 0x21, 7, 0, 0xD, 0x15, 4, 0, 1, 0, 0x1D, 0x10, 0, 0, 0, 0x25, 0x88, 0x1E, 0xAC, 0xF2,
		// ];
		// let h = MsgHead::decode(&head[..]);

		// assert_eq!(
		// 	h.unwrap(),
		// 	MsgHead {
		// 		magic: 218105633,
		// 		vertype: 65540,
		// 		len: 16,
		// 		crc32: 4071366280
		// 	}
		// );

		Ok(())
	}

	#[test]
	fn parse_multi() -> Result<()> {
		let msg = build_heartbeat_msg(11451_u16.into());
		println!("{:?}", msg);
		println!("msg: {:X?}", msg.encode_to_vec());

		let mut buf = bytes::BytesMut::with_capacity(128);
		msg.encode(&mut buf)?;
		msg.encode(&mut buf)?;
		println!("init buf: {:X?}", buf.to_vec());

		let mut parse = || -> Result<()> {
			let h = try_parse_head(&mut buf)?.unwrap();
			println!("{:X?}", buf.to_vec());
			let b = try_parse_body(&h, &mut buf)?.unwrap();
			println!("{:X?}", buf.to_vec());

			assert_eq!(Msg(h, b), msg);

			Ok(())
		};

		parse()?;
		parse()?;

		assert_eq!(buf.len(), 0);

		Ok(())
	}
}
