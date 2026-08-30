// ============= PRESETS =============
// Defines the 4 brightness presets that can be applied with a single command.

pub struct Preset {
    pub name: &'static str,
    pub brightness: u8,
    pub temp: u8,
}

impl Preset {
    pub const NIGHT: Self = Self {
        name: "night",
        brightness: 15,
        temp: 100,
    };
    pub const SUPERNIGHT: Self = Self {
        name: "supernight",
        brightness: 7,
        temp: 100,
    };
    pub const FULLNIGHT: Self = Self {
        name: "fullnight",
        brightness: 0,
        temp: 100,
    };
    pub const DAY: Self = Self {
        name: "day",
        brightness: 40,
        temp: 0,
    };
}

pub const PRESETS: [Preset; 4] = [
    Preset::NIGHT,
    Preset::SUPERNIGHT,
    Preset::FULLNIGHT,
    Preset::DAY,
];
