struct Bus {
    id: String,
    volume: f32,
    muted: bool,
}

impl Bus {
    pub fn new(bus_name: String) -> Self {
        Bus {
            id: bus_name,
            volume: 0.8,
            muted: false,
        }
    }
}
