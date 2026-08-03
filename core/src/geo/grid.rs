use std::sync::Arc;

use crate::geo::interpolation::bilinear_interpolation;

/// 2次元補間のための事前計算済みインデックス・比率
#[derive(Clone)]
pub struct GridInterpolationKey {
    pub(crate) lat_prev: usize,
    pub(crate) lat_next: usize,
    pub(crate) lon_prev: usize,
    pub(crate) lon_next: usize,
    pub(crate) ratio_lat: f64,
    pub(crate) ratio_lon: f64,
}

/// 2次元格子データ（1つの等圧面、1つの変数に対応）
#[derive(Clone)]
pub(crate) struct LatLonGrid {
    pub(crate) values: Vec<f32>,
    pub(crate) lon_coords: Arc<Vec<f32>>,
    pub(crate) lat_coords: Arc<Vec<f32>>,
    pub(crate) width: usize,
    pub(crate) height: usize,
}

impl LatLonGrid {
    /// ソート済み座標配列に対して2分探索でターゲットを挟む2つのインデックスを返すヘルパー関数
    /// 戻り値は (ターゲットより前のインデックス, ターゲットより後のインデックス)
    fn find_bracket(coords: &[f32], target: f32) -> (usize, usize) {
        let is_ascending = coords[0] < *coords.last().unwrap();

        let partition = if is_ascending {
            coords.partition_point(|&c| c < target)
        } else {
            coords.partition_point(|&c| c > target)
        };
        if partition == 0 || partition >= coords.len() {
            (0, 0)
        } else {
            (partition - 1, partition)
        }
    }

    /// 2インデックス間の補間比率 (0.0〜1.0) を算出するヘルパー関数
    /// 経度方向では 0°/360° の巡回を考慮する
    fn calc_ratio(
        coords: &[f32],
        prev_index: usize,
        next_index: usize,
        target: f32,
        wrap_360: bool,
    ) -> f64 {
        if prev_index == next_index {
            return 0.0;
        }
        let coord_prev = coords[prev_index] as f64;
        let coord_next = coords[next_index] as f64;
        let target_f64 = target as f64;
        let (span, offset_from_prev) = if wrap_360 && coord_next < coord_prev {
            (
                (coord_next + 360.0) - coord_prev,
                if target_f64 < coord_prev {
                    target_f64 + 360.0
                } else {
                    target_f64
                } - coord_prev,
            )
        } else {
            (coord_next - coord_prev, target_f64 - coord_prev)
        };
        (offset_from_prev / span).clamp(0.0, 1.0)
    }

    /// 端の座標で丸めるヘルパー関数
    fn clamp_lat_index(coords: &[f32], height: usize, target: f32) -> usize {
        let top_is_first = coords[0] > coords[height - 1];
        let max_val = coords[0].max(coords[height - 1]);
        if target > max_val {
            if top_is_first { 0 } else { height - 1 }
        } else if top_is_first {
            height - 1
        } else {
            0
        }
    }

    /// 経度が0-360度まである場合は端同士が隣接するので折り返すヘルパー関数
    fn clamp_lon_index(coords: &[f32], width: usize, target: f32) -> (usize, usize) {
        let is_global = coords[0] <= 1.0 && coords[width - 1] >= 358.0;
        if is_global {
            (width - 1, 0)
        } else {
            let idx = if target < coords[0] { 0 } else { width - 1 };
            (idx, idx)
        }
    }

    /// 任意の緯度・経度における2次元線形補間値を返す
    #[allow(dead_code)]
    pub(crate) fn interpolate_at(&self, lat: f64, lon: f64) -> f64 {
        let target_lat = lat as f32;
        let target_lon = (lon as f32).rem_euclid(360.0);

        let (lat_prev, lat_next) = {
            let (prev, next) = Self::find_bracket(&self.lat_coords, target_lat);
            if prev == next {
                let clamped = Self::clamp_lat_index(&self.lat_coords, self.height, target_lat);
                (clamped, clamped)
            } else {
                (prev, next)
            }
        };

        let (lon_prev, lon_next) = {
            let (prev, next) = Self::find_bracket(&self.lon_coords, target_lon);
            if prev == next {
                Self::clamp_lon_index(&self.lon_coords, self.width, target_lon)
            } else {
                (prev, next)
            }
        };

        let ratio_lat = Self::calc_ratio(&self.lat_coords, lat_prev, lat_next, target_lat, false);
        let ratio_lon = Self::calc_ratio(&self.lon_coords, lon_prev, lon_next, target_lon, true);

        let v00 = self.values[lat_prev * self.width + lon_prev] as f64;
        let v01 = self.values[lat_prev * self.width + lon_next] as f64;
        let v10 = self.values[lat_next * self.width + lon_prev] as f64;
        let v11 = self.values[lat_next * self.width + lon_next] as f64;

        bilinear_interpolation(v00, v01, v10, v11, ratio_lon, ratio_lat)
    }

    /// 緯度・経度の二分探索結果を事前計算して返す
    pub(crate) fn compute_interpolation_key(&self, lat: f64, lon: f64) -> GridInterpolationKey {
        let target_lat = lat as f32;
        let target_lon = (lon as f32).rem_euclid(360.0);

        let (lat_prev, lat_next) = {
            let (prev, next) = Self::find_bracket(&self.lat_coords, target_lat);
            if prev == next {
                let clamped = Self::clamp_lat_index(&self.lat_coords, self.height, target_lat);
                (clamped, clamped)
            } else {
                (prev, next)
            }
        };

        let (lon_prev, lon_next) = {
            let (prev, next) = Self::find_bracket(&self.lon_coords, target_lon);
            if prev == next {
                Self::clamp_lon_index(&self.lon_coords, self.width, target_lon)
            } else {
                (prev, next)
            }
        };

        let ratio_lat = Self::calc_ratio(&self.lat_coords, lat_prev, lat_next, target_lat, false);
        let ratio_lon = Self::calc_ratio(&self.lon_coords, lon_prev, lon_next, target_lon, true);

        GridInterpolationKey {
            lat_prev,
            lat_next,
            lon_prev,
            lon_next,
            ratio_lat,
            ratio_lon,
        }
    }

    /// 事前計算したインデックス・比率を使って2次元線形補間値を返す
    pub(crate) fn interpolate_with_key(&self, key: &GridInterpolationKey) -> f64 {
        let v00 = self.values[key.lat_prev * self.width + key.lon_prev] as f64;
        let v01 = self.values[key.lat_prev * self.width + key.lon_next] as f64;
        let v10 = self.values[key.lat_next * self.width + key.lon_prev] as f64;
        let v11 = self.values[key.lat_next * self.width + key.lon_next] as f64;

        bilinear_interpolation(v00, v01, v10, v11, key.ratio_lon, key.ratio_lat)
    }
}
