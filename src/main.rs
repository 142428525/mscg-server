use std::env;

use anyhow::{Result, bail};

use mscg_server::*;

fn main() -> Result<()> {
	let args: Vec<String> = env::args().collect();

	let (player_num, port): (u8, u16) = match args.len() {
		2 if matches!(args[1].parse::<u8>(), Ok(x) if x > 0) => {
			println!(
				"No argument provided, so using the default port ({}) instead.",
				server::DEFAULT_PORT
			);
			(args[1].parse().unwrap(), server::DEFAULT_PORT)
		}
		3 if matches!(args[1].parse::<u8>(), Ok(x) if x > 0)
			&& matches!(args[2].parse::<u16>(), Ok(x) if (1024..49152_u16).contains(&x)) =>
		{
			(args[1].parse().unwrap(), args[2].parse().unwrap())
		}
		_ => {
			bail!(
				"Please provide the player count maximum (1 ~ 255), or an optional port number (1024 ~ 49151) only."
			);
		}
	};

	let mut s = server::Server::new(player_num);
	s.listen(port)?;

	Ok(())
}
