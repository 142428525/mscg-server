#[derive(Debug)]
pub struct Cell {
	_terrain: i32,
	_basic: i32,
	_structual: i32,
}

impl Cell {
	fn new() -> Self {
		Self {
			_terrain: 0,
			_basic: 1,
			_structual: 2,
		}
	}
}

type Chunk = [[Cell; 32]; 32];

pub struct Map {
	pub data: Vec<Chunk>,
}

impl Map {
	fn new() -> Self {
		Self { data: Vec::new() }
	}

	pub fn create_chunk(&mut self) -> () {
		let tmp: [[_; 32]; 32] = std::array::from_fn(|_| std::array::from_fn(|_| Cell::new()));
		self.data.push(tmp);
	}
}

pub struct GameState {
	pub map: Map,
}

impl GameState {
	pub fn new() -> Self {
		Self { map: Map::new() }
	}
}

fn _w() {
	let mut w = GameState::new();
	w.map.create_chunk();
	let v = &w.map.data[0][0][0];
	println!("{:?}", v);
}
