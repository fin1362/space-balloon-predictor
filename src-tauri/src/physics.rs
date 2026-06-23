// 乾燥空気の比気体定数R_d
// 気体定数をR, 気体の分子量をMとおくと，R_d=R/Mと表される
const GAS_CONSTANT_DRY_AIR: f64 = 287.058;

/// 風速ベクトル (東西成分u, 南北成分v)
#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub struct WindVector {
    pub u: f64, // 東西風 (正の値が東向き、負の値が西向き)
    pub v: f64, // 南北風 (正の値が北向き、負の値が南向き)
}

/// 特定の気圧と気温から乾燥空気の密度を計算する
/// PM = dRTより, d = (PM)/(RT) = P/(R_d*T)
/// pressure_pa: 気圧 (Pa)
/// temperature_k: 気温 (K)
pub fn air_density(pressure_pa: f64, temperature_k: f64) -> f64 {
    if temperature_k <= 0.0 {
        return 0.0;
    }
    pressure_pa / (GAS_CONSTANT_DRY_AIR * temperature_k)
}

/// 空気密度の比率に基づいて、ある高度での下降終端速度を推定する
/// velocity_0: 地上での終端速度 (m/s)
/// density_0: 地上での空気密度 (kg/m^3)
/// density: 推定したい高度における空気密度 (kg/m^3)
pub fn terminal_velocity(velocity_0: f64, density_0: f64, density_z: f64) -> f64 {
    if density_z <= 0.0 {
        return velocity_0;
    }
    velocity_0 * (density_0 / density_z).sqrt()
}

/// 理想大気モデル（国際標準大気：ISA / 米国標準大気1976モデル）に基づいて
/// 任意の海抜高度(m)における標準的な空気密度(kg/m^3)を算出する
/// 気象データの範囲外に出たときにフォールバックとして利用する
pub fn standard_atmosphere_density(altitude_m: f64) -> f64 {
    let h = altitude_m.max(0.0);

    let (pressure_pa, temperature_k) = if h <= 11000.0 {
        // 対流圏: 地上〜高度11km
        let temp = 288.15 - 0.0065 * h;
        let press = 101325.0 * (temp / 288.15).powf(5.25588);
        (press, temp)
    } else if h <= 20000.0 {
        // 成層圏下部: 高度11km〜20km
        let temp = 216.65;
        let press = 22632.1 * (-0.00015769 * (h - 11000.0)).exp();
        (press, temp)
    } else if h <= 32000.0 {
        // 成層圏中部: 高度20km〜32km
        let temp = 216.65 + 0.001 * (h - 20000.0);
        let press = 5474.89 * (216.65 / temp).powf(34.1631);
        (press, temp)
    } else {
        // 32km以上
        let temp = 228.65;
        let press = 868.0 * (-0.00014 * (h - 32000.0)).exp();
        (press, temp)
    };

    air_density(pressure_pa, temperature_k)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_air_density() {
        // 標準大気圧 101325 Pa, 気温 288.15 K の時の密度は約 1.225 kg/m^3
        // https://pigeon-poppo.com/standard-atmosphere/
        let rho = air_density(101325.0, 288.15);
        assert!((rho - 1.225).abs() < 0.01);
    }

    #[test]
    fn test_terminal_velocity() {
        let velocity_0 = 5.0; // 地上での終端速度 5 m/s
        let density_0 = 1.225; // 地上空気密度
        let density = 0.30625; // 密度の薄い高度（地上の4分の1）

        // 密度が4分の1になると、終端速度は2倍（10.0 m/s）になるはず
        let velocity = terminal_velocity(velocity_0, density_0, density);
        assert!((velocity - 10.0).abs() < 1e-5);
    }
}
