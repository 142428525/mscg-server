use std::{
	env,
	io::{Read, Write},
	net::{SocketAddr, TcpStream},
	thread,
	time::Duration,
};

use anyhow::{Result, bail};
use prost::Message;

use mscg_server::*;

fn main() -> Result<()> {
	let args: Vec<String> = env::args().collect();

	let addr: SocketAddr = match args.len() {
		1 => {
			let host = format!("127.0.0.1:{}", server::DEFAULT_PORT);
			println!("Default host: {host}");
			host.parse().unwrap()
		}
		2 if args[1].parse::<SocketAddr>().is_ok() => args[1].parse().unwrap(),
		_ => {
			bail!("Please provide a valid socket address.");
		}
	};

	let mut stream = loop {
		println!("Waiting for connecting to {addr}.");
		if let Ok(x) = TcpStream::connect_timeout(&addr, Duration::from_secs(5)) {
			break x;
		}
	};

	println!("Here: {:?}", stream.local_addr());

	loop {
		let hb = packets::build_heartbeat_msg(0_u8.into());

		let mut send_msg =|| -> Result<()> {
			stream.write_all(&hb.0.encode_to_vec())?;
			stream.write_all(&hb.1.encode_to_vec())?;

			stream.flush()?;
			Ok(())
		};
		if let Err(e) = send_msg() {
			return Err(e);
		}

		println!("Message sent: {:?}", hb);

		// let mut buf = bytes::BytesMut::with_capacity(64);
		// buf.resize(64, 0);
		// stream.read_exact(&mut buf)?;
		// println!("recv: data: {:X?}", buf);

		thread::sleep(Duration::from_secs(5));
	}

	//
	// let mut s = String::new();
	// std::io::stdin().read_line(&mut s)?;
	// stream.write(&s.as_bytes())?;
	//

	//Ok(())
}
