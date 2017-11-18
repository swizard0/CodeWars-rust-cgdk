
#[derive(Clone, Debug)]
pub struct Derivatives {
    pub d_x: f64,
    pub d_y: f64,
    pub d_durability: i32,
}

impl Derivatives {
    pub fn new() -> Derivatives {
        Derivatives {
            d_x: 0.,
            d_y: 0.,
            d_durability: 0,
        }
    }

    pub fn clear(&mut self) {
        self.d_x = 0.;
        self.d_y = 0.;
        self.d_durability = 0;
    }
}
