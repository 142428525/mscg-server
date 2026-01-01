use std::{
	io::{Read, Write},
	net, sync, thread, time,
};

use anyhow::{Result, bail};
use tklog::LOG;

use crate::*;

mod logic;

static INIT: sync::Once = sync::Once::new();

pub const DEFAULT_PORT: u16 = 27315;

#[derive(Debug)]
struct Connection {
	socket: net::SocketAddr,
	stream: net::TcpStream, //io::BufWriter<net::TcpStream>,
	read_buf: bytes::BytesMut,
}

impl Connection {
	fn new(stream: net::TcpStream) -> Self {
		let mut buf = bytes::BytesMut::with_capacity(4096);
		buf.resize(4096, 0);
		Self {
			socket: stream.peer_addr().expect("Failed to get peer's addr"),
			stream, //: io::BufWriter::new(stream),
			read_buf: buf,
		}
	}

	/// Ok(None) means all data has been decoded successfully, and we've met the end of the stream.
	fn read_msg(&mut self) -> Result<Option<packet::msg::Msg>> {
		fn read_from_stream(stream: &mut net::TcpStream, buf: &mut bytes::BytesMut) -> Result<usize> {
			let read_bytes_num = stream /*.get_mut()*/
				.read(&mut buf[..])?;

			log::debug!(
				"{} byte(s) read, peeking ..16 of buf: {:X?}",
				read_bytes_num,
				buf.get(..16)
			);

			Ok(read_bytes_num)
		}

		let head = loop {
			match packet::try_parse_head(&mut self.read_buf) {
				Ok(Some(x)) => break x,
				Err(e) => log::error!("{}", e),
				Ok(None) => continue,
			}

			let read_bytes_num = read_from_stream(&mut self.stream, &mut self.read_buf)?;
			if read_bytes_num == 0 {
				if self.read_buf.is_empty() {
					return Ok(None);
				} else {
					bail!("Connection closed by peer. Incomplete data.");
				}
			}
		};

		let body = loop {
			match packet::try_parse_body(&head, &mut self.read_buf) {
				Ok(Some(x)) => break x,
				Err(e) => log::error!("{}", e),
				Ok(None) => continue,
			}

			let read_bytes_num = read_from_stream(&mut self.stream, &mut self.read_buf)?;
			if read_bytes_num == 0 {
				if self.read_buf.is_empty() {
					return Ok(None);
				} else {
					bail!("Connection closed by peer. Incomplete data.");
				}
			}
		};

		Ok(Some(packet::msg::Msg(head, body)))
	}
}

impl From<net::TcpStream> for Connection {
	fn from(value: net::TcpStream) -> Self {
		Self::new(value)
	}
}

pub struct Server {
	max_player_num: u8,
	players: Vec<Connection>,
}

impl Server {
	pub fn new(player_num: u8) -> Self {
		INIT.call_once(|| {
			LOG.set_formatter("{level}\t{time} @ {file}: {message}\n").uselog(); // guarantee that all log::* calls happen after here
		});

		Self {
			max_player_num: player_num,
			players: Vec::with_capacity(player_num as usize),
		}
	}

	pub fn listen(&mut self, port: u16) -> Result<()> {
		let listener = net::TcpListener::bind(format!("127.0.0.1:{port}"))?;
		log::info!("Listening to local port {port}...");

		for stream in listener.incoming() {
			let stream = stream?;

			//
			if self.players.len() < self.max_player_num as usize {
				if let Err(e) = self.handle(stream) {
					log::error!("{}", e);
				}
			}
			//
		}

		Ok(())
	}

	fn handle(&mut self, stream: net::TcpStream) -> Result<()> {
		self.players.push(stream.into());

		//
		log::info!(
			"Player #{}: {}",
			self.players.len() - 1,
			self.players.last().unwrap().socket
		);
		//

		let peer = self.players.last_mut().unwrap();

		loop {
			let msg = peer.read_msg()?;
			log::debug!("recv msg: {:?}", msg);

			// let hb = packets_helper::build_heartbeat_msg();

			// peer.stream.write_all(&hb.0.encode_to_vec())?;
			// peer.stream.write_all(&hb.1.encode_to_vec())?;

			// peer.stream.flush()?;

			// println!(
			// 	"send to peer: {} / {:?}",
			// 	peer.get_socket_str(),
			// 	peer.stream.peer_addr()
			// );

			thread::sleep(time::Duration::from_secs(3));
		}

		//Ok(())
	}
}
