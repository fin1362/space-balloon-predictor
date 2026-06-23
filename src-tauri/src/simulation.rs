use crate::coords::Geodetic;
use crate::grib_reader::Atmosphere;
use crate::interpolation::lerp;
use crate::physics::{WindVector, air_density, standard_atmosphere_density, terminal_velocity};

const SEA_LEVEL_DENSITY: f64 = 1.225;

/// 位置の変化率（度/秒、メートル/秒）
#[derive(Default, Debug, Clone, Copy)]
struct StateRate {
    dlat: f64,
    dlon: f64,
    dalt: f64,
}

impl std::ops::Add for StateRate {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Self {
            dlat: self.dlat + other.dlat,
            dlon: self.dlon + other.dlon,
            dalt: self.dalt + other.dalt,
        }
    }
}

impl std::ops::Mul<f64> for StateRate {
    type Output = Self;
    fn mul(self, scalar: f64) -> Self {
        Self {
            dlat: self.dlat * scalar,
            dlon: self.dlon * scalar,
            dalt: self.dalt * scalar,
        }
    }
}

/// バルーンの現在の状態（緯度・経度・高度で直接管理）
#[derive(Debug, Clone, Copy)]
pub struct BalloonState {
    pub lat: f64,
    pub lon: f64,
    pub alt: f64,
    pub time: f64,
    pub is_burst: bool,
}

impl Default for BalloonState {
    fn default() -> Self {
        Self {
            lat: 0.0,
            lon: 0.0,
            alt: 0.0,
            time: 0.0,
            is_burst: false,
        }
    }
}

/// シミュレーションの結果を保持する
#[derive(Debug, Clone)]
pub struct Trajectory {
    pub states: Vec<BalloonState>,
}

impl Trajectory {
    pub fn new(states: Vec<BalloonState>) -> Self {
        Self { states }
    }

    /// 成層圏（高度11,000m以上）に滞在した時間（秒）を返す
    pub fn stratosphere_duration_s(&self) -> f64 {
        const TROPOPAUSE_M: f64 = 11_000.0;
        let mut enter_time: Option<f64> = None;
        let mut total = 0.0;

        for w in self.states.windows(2) {
            let a = &w[0];
            let b = &w[1];
            let a_above = a.alt >= TROPOPAUSE_M;
            let b_above = b.alt >= TROPOPAUSE_M;

            if a_above && enter_time.is_none() {
                enter_time = Some(a.time);
            }

            if let Some(t0) = enter_time {
                if !b_above {
                    total += b.time - t0;
                    enter_time = None;
                }
            }
        }

        if let Some(t0) = enter_time {
            if let Some(last) = self.states.last() {
                total += last.time - t0;
            }
        }

        total
    }
}

/// シミュレーションの設定項目
#[derive(Debug, Clone)]
pub struct SimConfig {
    pub launch_site: Geodetic,
    pub ascent_rate_m_s: f64,
    pub ground_descend_rate_m_s: f64,
    pub burst_altitude_m: f64,
    pub dt: f64,
}

/// 3時刻のGRIB2データと座標変換を保持し、RK4で軌道を計算するシミュレータ
pub struct Simulator {
    config: SimConfig,
    // スタート前の予報データ (例: f018)
    weather_earliest: Atmosphere,
    // その3時間後の予報データ (例: f021)
    weather_middle: Atmosphere,
    // その6時間後の予報データ (例: f024)
    weather_latest: Atmosphere,
    launch_offset_hours: f64,
}

/// StateRate に基づいてバルーン状態を dt 分だけ進める
fn apply_rate(state: &BalloonState, rate: &StateRate, dt: f64) -> BalloonState {
    BalloonState {
        lat: state.lat + rate.dlat * dt,
        lon: state.lon + rate.dlon * dt,
        alt: state.alt + rate.dalt * dt,
        time: state.time + dt,
        is_burst: state.is_burst,
    }
}

impl Simulator {
    pub fn new(
        config: SimConfig,
        weather_earliest: Atmosphere,
        weather_middle: Atmosphere,
        weather_latest: Atmosphere,
        launch_offset_hours: f64,
    ) -> Self {
        Self {
            config,
            weather_earliest,
            weather_middle,
            weather_latest,
            launch_offset_hours,
        }
    }

    /// 現在の状態から緯度・経度・高度の変化率 (deg/s, m/s) を計算する
    fn dynamics(&self, state: &BalloonState) -> StateRate {
        let geo = Geodetic {
            lat: state.lat,
            lon: state.lon,
            alt: state.alt,
        };

        let current_elapsed_hours = self.launch_offset_hours + (state.time / 3600.0);

        // 3つのデータのうち、どの2つの間を補間するか決定し、その比率(0.0〜1.0)を算出
        let (env_prev, env_next, time_ratio) = if current_elapsed_hours <= 3.0 {
            let ratio = (current_elapsed_hours / 3.0).clamp(0.0, 1.0);
            (&self.weather_earliest, &self.weather_middle, ratio)
        } else {
            let ratio = ((current_elapsed_hours - 3.0) / 3.0).clamp(0.0, 1.0);
            (&self.weather_middle, &self.weather_latest, ratio)
        };

        let key = env_prev
            .compute_interpolation_key(geo.lat, geo.lon)
            .unwrap();

        // それぞれの気圧面データを取得
        let alt_map_prev = env_prev.build_altitude_map_with_key(&key);
        let alt_map_next = env_next.build_altitude_map_with_key(&key);
        let prev_state = env_prev.get_atmospheric_state_with_map(&geo, &alt_map_prev, &key);
        let next_state = env_next.get_atmospheric_state_with_map(&geo, &alt_map_next, &key);

        // 水平風速の決定
        // 前後の時刻の気圧面データの有無に応じて、風速を補間する
        let wind = match (&prev_state, &next_state) {
            // 両方の時刻にデータがある場合は線形補間
            (Some(prev), Some(next)) => {
                let u = lerp(prev.wind.u, next.wind.u, time_ratio);
                let v = lerp(prev.wind.v, next.wind.v, time_ratio);
                WindVector { u, v }
            }
            (Some(prev), None) => prev.wind,
            (None, Some(next)) => next.wind,
            // データがない場合は無風を仮定
            _ => WindVector::default(),
        };

        // 垂直速度の決定
        let vertical_velocity = if !state.is_burst {
            self.config.ascent_rate_m_s
        } else {
            // 破裂後の落下速度を求めるため、高度における空気密度を補間する
            // 気圧と気温から密度を算出し、前後時刻で線形補間
            let density_at_altitude = match (&prev_state, &next_state) {
                // 両方の時刻にデータがある場合は密度を補間
                (Some(prev), Some(next)) => {
                    let density_prev = air_density(prev.pressure_pa, prev.temperature_k);
                    let density_next = air_density(next.pressure_pa, next.temperature_k);
                    lerp(density_prev, density_next, time_ratio)
                }
                // 片方 only 場合はその時刻の密度を使用
                (Some(prev), None) => air_density(prev.pressure_pa, prev.temperature_k),
                (None, Some(next)) => air_density(next.pressure_pa, next.temperature_k),
                _ => standard_atmosphere_density(state.alt),
            };

            let speed = terminal_velocity(
                self.config.ground_descend_rate_m_s,
                SEA_LEVEL_DENSITY,
                density_at_altitude,
            );
            -speed
        };

        let lat_rad = state.lat.to_radians();
        let sin_lat = lat_rad.sin();

        const WGS84_A: f64 = 6_378_137.0;       // 赤道半径（地球長半径a）
        const WGS84_E2: f64 = 0.00669437999014; // 第一離心率の2乗

        let w = (1.0 - WGS84_E2 * sin_lat * sin_lat).sqrt();
        let m = WGS84_A * (1.0 - WGS84_E2) / (w * w * w); // 子午線曲率半径 (南北方向)
        let n = WGS84_A / w;                             // 卯酉線曲率半径 (東西方向)

        let deg_per_rad = 180.0 / std::f64::consts::PI;

        let dlat_dt = wind.v / (m + state.alt) * deg_per_rad;
        let dlon_dt = wind.u / ((n + state.alt) * lat_rad.cos()) * deg_per_rad;

        StateRate {
            dlat: dlat_dt,
            dlon: dlon_dt,
            dalt: vertical_velocity,
        }
    }

    /// 4次のルンゲクッタ法で1ステップ進める
    fn rk4_step(&self, state: &BalloonState, dt: f64) -> BalloonState {
        let k1 = self.dynamics(state);
        let k2 = self.dynamics(&apply_rate(state, &k1, dt * 0.5));
        let k3 = self.dynamics(&apply_rate(state, &k2, dt * 0.5));
        let k4 = self.dynamics(&apply_rate(state, &k3, dt));

        let delta = (k1 + k2 * 2.0 + k3 * 2.0 + k4) * (dt / 6.0);

        BalloonState {
            lat: state.lat + delta.dlat,
            lon: state.lon + delta.dlon,
            alt: state.alt + delta.dalt,
            time: state.time + dt,
            is_burst: state.is_burst,
        }
    }

    pub fn run(&self) -> Trajectory {
        let mut trajectory = Vec::new();

        let mut state = BalloonState {
            lat: self.config.launch_site.lat,
            lon: self.config.launch_site.lon,
            alt: self.config.launch_site.alt,
            time: 0.0,
            is_burst: false,
        };

        trajectory.push(state);

        while !(state.is_burst && state.alt <= self.config.launch_site.alt) {
            let mut next_state = self.rk4_step(&state, self.config.dt);

            if !state.is_burst && next_state.alt >= self.config.burst_altitude_m {
                next_state.is_burst = true;
            } else {
                next_state.is_burst = state.is_burst;
            }

            state = next_state;
            trajectory.push(state);
        }

        Trajectory::new(trajectory)
    }
}
