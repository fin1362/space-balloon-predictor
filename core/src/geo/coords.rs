/// 緯度・経度・高度を表す座標
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Geodetic {
    pub lat: f64,
    pub lon: f64,
    pub alt: f64,
}

/// 地球の平均半径 (m)
/// https://en.wikipedia.org/wiki/Earth_radius
pub const EARTH_RADIUS: f64 = 6_371_000.0;
