/// 1次元の線形補完
pub fn lerp(v0: f64, v1: f64, ratio: f64) -> f64 {
    v0 + (v1 - v0) * ratio
}

/// 2次元の線形補間
pub fn bilinear_interpolation(
    v00: f64,
    v01: f64,
    v10: f64,
    v11: f64,
    ratio_x: f64,
    ratio_y: f64,
) -> f64 {
    let top = lerp(v00, v01, ratio_x);
    let bottom = lerp(v10, v11, ratio_x);
    lerp(top, bottom, ratio_y)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lerp() {
        assert!((lerp(10.0, 20.0, 0.0) - 10.0).abs() < 1e-9);
        assert!((lerp(10.0, 20.0, 1.0) - 20.0).abs() < 1e-9);
        assert!((lerp(10.0, 20.0, 0.5) - 15.0).abs() < 1e-9);
    }

    #[test]
    fn test_bilinear_interpolation() {
        // 4隅の値が既知のとき、端点・中央値が正しいか
        //  v00=0  v01=10
        //  v10=20 v11=30
        assert!((bilinear_interpolation(0.0, 10.0, 20.0, 30.0, 0.0, 0.0) - 0.0).abs() < 1e-9);
        assert!((bilinear_interpolation(0.0, 10.0, 20.0, 30.0, 1.0, 0.0) - 10.0).abs() < 1e-9);
        assert!((bilinear_interpolation(0.0, 10.0, 20.0, 30.0, 0.0, 1.0) - 20.0).abs() < 1e-9);
        assert!((bilinear_interpolation(0.0, 10.0, 20.0, 30.0, 1.0, 1.0) - 30.0).abs() < 1e-9);
        // 中央点は4値の平均
        assert!((bilinear_interpolation(0.0, 10.0, 20.0, 30.0, 0.5, 0.5) - 15.0).abs() < 1e-9);
    }
}
