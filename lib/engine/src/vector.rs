#[derive(Default, Debug, Clone, PartialEq)]
pub struct FVector3d {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl FVector3d {
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    pub fn set(&mut self, x: f64, y: f64, z: f64) {
        self.x = x;
        self.y = y;
        self.z = z;
    }

    pub fn is_zero(&self) -> bool {
        matches!(
            self,
            Self {
                x: 0.0,
                y: 0.0,
                z: 0.0
            }
        )
    }

    pub fn get_abs_max(&self) -> f64 {
        [self.x.abs(), self.y.abs(), self.z.abs()]
            .into_iter()
            .fold(0.0, |a, b| a.max(b))
    }

    pub fn get_abs_min(&self) -> f64 {
        [self.x.abs(), self.y.abs(), self.z.abs()]
            .into_iter()
            .fold(0.0, |a, b| a.min(b))
    }
}
