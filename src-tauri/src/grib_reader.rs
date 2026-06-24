use grib::{Grib2SubmessageDecoder, LatLons};
use rayon::prelude::*;
use std::collections::BTreeMap;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::Arc;

use crate::coords::Geodetic;
use crate::grid::{GridInterpolationKey, LatLonGrid};
use crate::interpolation::lerp;
use crate::physics::WindVector;

/// GRIB2パラメータ識別子
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct GribParameter {
    pub discipline: u8,
    pub category: u8,
    pub number: u8,
}

const PARAM_U: GribParameter = GribParameter {
    discipline: 0,
    category: 2,
    number: 2,
}; // 東西風 (m/s)
const PARAM_V: GribParameter = GribParameter {
    discipline: 0,
    category: 2,
    number: 3,
}; // 南北風 (m/s)
const PARAM_T: GribParameter = GribParameter {
    discipline: 0,
    category: 0,
    number: 0,
}; // 気温 (K)
const PARAM_H: GribParameter = GribParameter {
    discipline: 0,
    category: 3,
    number: 5,
}; // ジオポテンシャル高度 (gpm)

/// GRIB2 グリッドの緯度・経度座標軸
#[derive(Clone)]
struct GridMetadata {
    lon_coords: Arc<Vec<f32>>,
    lat_coords: Arc<Vec<f32>>,
    width: usize,
    height: usize,
}

/// GRIB サブメッセージは変数単位で順不同に現れるため、
/// すべて揃った時点で `AtmosphereLayer` に変換する
#[derive(Default)]
struct PressureLevelBuilder {
    u_wind: Option<LatLonGrid>,
    v_wind: Option<LatLonGrid>,
    temp_k: Option<LatLonGrid>,
    height_gpm: Option<LatLonGrid>,
}

/// 1等圧面の気圧値 (Pa) と、補間された位勢高度 (m) のペア
pub struct PressureHeightPair {
    pub pressure_pa: i32,
    pub altitude_m: f64,
}

/// GRIB2 ファイルから解析された全等圧面の気象データ
/// キーは気圧値 (Pa)、値は対応するAtmosphereLayer
pub struct Atmosphere {
    levels: BTreeMap<i32, AtmosphereLayer>,
}

/// 1等圧面の気象データ
struct AtmosphereLayer {
    u_wind: LatLonGrid,
    v_wind: LatLonGrid,
    temp_k: LatLonGrid,
    height_gpm: LatLonGrid,
}

/// 1地点の大気状態 (風速・気温・気圧)
pub struct AtmospherePoint {
    pub wind: WindVector,
    pub temperature_k: f64,
    pub pressure_pa: f64,
}

impl Atmosphere {
    /// 最初の等圧面のグリッドを使って緯度・経度の補間キーを計算する
    pub fn compute_interpolation_key(&self, lat: f64, lon: f64) -> Option<GridInterpolationKey> {
        self.levels
            .values()
            .next()
            .map(|level| level.height_gpm.compute_interpolation_key(lat, lon))
    }

    /// 全等圧面の高度を2次元補間して高度順にソートしたマップを返す
    #[allow(dead_code)]
    pub fn build_altitude_map(&self, lat: f64, lon: f64) -> Vec<PressureHeightPair> {
        let mut map: Vec<PressureHeightPair> = self
            .levels
            .iter()
            .map(|(&press, level_data)| PressureHeightPair {
                pressure_pa: press,
                altitude_m: level_data.height_gpm.interpolate_at(lat, lon),
            })
            .collect();
        map.sort_by(|a, b| {
            a.altitude_m
                .partial_cmp(&b.altitude_m)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        map
    }

    /// 事前計算済みの補間キーを使って全等圧面の高度マップを構築する
    pub fn build_altitude_map_with_key(
        &self,
        key: &GridInterpolationKey,
    ) -> Vec<PressureHeightPair> {
        let mut map: Vec<PressureHeightPair> = self
            .levels
            .iter()
            .map(|(&press, level_data)| PressureHeightPair {
                pressure_pa: press,
                altitude_m: level_data.height_gpm.interpolate_with_key(key),
            })
            .collect();
        map.sort_by(|a, b| {
            a.altitude_m
                .partial_cmp(&b.altitude_m)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        map
    }

    /// 事前構築済みの高度マップと補間キーを使って大気状態を取得する
    pub fn get_atmospheric_state_with_map(
        &self,
        geo: &Geodetic,
        alt_map: &[PressureHeightPair],
        key: &GridInterpolationKey,
    ) -> Option<AtmospherePoint> {
        if alt_map.is_empty() {
            return None;
        }

        let (p_lower, p_upper, vertical_ratio) = Self::find_vertical_bracket(alt_map, geo.alt);

        let data_low = self.levels.get(&p_lower)?;
        let data_high = self.levels.get(&p_upper)?;

        let u = lerp(
            data_low.u_wind.interpolate_with_key(key),
            data_high.u_wind.interpolate_with_key(key),
            vertical_ratio,
        );
        let v = lerp(
            data_low.v_wind.interpolate_with_key(key),
            data_high.v_wind.interpolate_with_key(key),
            vertical_ratio,
        );
        let temp = lerp(
            data_low.temp_k.interpolate_with_key(key),
            data_high.temp_k.interpolate_with_key(key),
            vertical_ratio,
        );
        let press = lerp(p_lower as f64, p_upper as f64, vertical_ratio);

        Some(AtmospherePoint {
            wind: WindVector { u, v },
            temperature_k: temp,
            pressure_pa: press,
        })
    }

    /// ソート済み高度マップに対して2分探索で等圧面ペア (lower, upper, 補間比率) を返す
    /// モデル範囲外の場合は端の面で丸める
    fn find_vertical_bracket(
        alt_map: &[PressureHeightPair],
        altitude: f64,
    ) -> (i32, i32, f64) {
        let partition = alt_map.partition_point(|a| a.altitude_m < altitude);
        match partition {
            // 最低高度より下なら最も低い等圧面の情報を使う
            0 => {
                let pressure = alt_map[0].pressure_pa;
                (pressure, pressure, 0.0)
            }
            // 最高高度より上なら最も高い等圧面の情報を使う
            i if i >= alt_map.len() => {
                let last = alt_map.len() - 1;
                let pressure = alt_map[last].pressure_pa;
                (pressure, pressure, 1.0)
            }
            // 正常に2つの等圧面で挟み込めた
            i => {
                let alt_lower = alt_map[i - 1].altitude_m;
                let alt_upper = alt_map[i].altitude_m;
                let ratio = (altitude - alt_lower) / (alt_upper - alt_lower);
                (alt_map[i - 1].pressure_pa, alt_map[i].pressure_pa, ratio)
            }
        }
    }

    /// 任意の緯度・経度・高度における気象環境を取得（Trilinear補間）
    #[allow(dead_code)]
    pub fn get_atmospheric_state_at(&self, geo: &Geodetic) -> Option<AtmospherePoint> {
        if self.levels.is_empty() {
            return None;
        }

        let alt_map = self.build_altitude_map(geo.lat, geo.lon);
        let (p_lower, p_upper, vertical_ratio) = Self::find_vertical_bracket(&alt_map, geo.alt);

        let data_low = self.levels.get(&p_lower)?;
        let data_high = self.levels.get(&p_upper)?;

        let u = lerp(
            data_low.u_wind.interpolate_at(geo.lat, geo.lon),
            data_high.u_wind.interpolate_at(geo.lat, geo.lon),
            vertical_ratio,
        );
        let v = lerp(
            data_low.v_wind.interpolate_at(geo.lat, geo.lon),
            data_high.v_wind.interpolate_at(geo.lat, geo.lon),
            vertical_ratio,
        );
        let temp = lerp(
            data_low.temp_k.interpolate_at(geo.lat, geo.lon),
            data_high.temp_k.interpolate_at(geo.lat, geo.lon),
            vertical_ratio,
        );
        let press = lerp(p_lower as f64, p_upper as f64, vertical_ratio);

        Some(AtmospherePoint {
            wind: WindVector { u, v },
            temperature_k: temp,
            pressure_pa: press,
        })
    }

    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let f = File::open(&path)?;
        let reader = BufReader::new(f);
        let grib = grib::from_reader(reader)?;

        let mut grid_metadata: Option<GridMetadata> = None;
        let mut decode_tasks: Vec<(GribParameter, i32, Grib2SubmessageDecoder)> = Vec::new();

        for (_index, submessage) in grib.iter() {
            let discipline = submessage.indicator().discipline;
            let prod_def = submessage.prod_def();
            let parameter_category = match prod_def.parameter_category() {
                Some(c) => c,
                None => continue,
            };
            let parameter_number = match prod_def.parameter_number() {
                Some(n) => n,
                None => continue,
            };

            let param = GribParameter {
                discipline,
                category: parameter_category,
                number: parameter_number,
            };

            if param != PARAM_U && param != PARAM_V && param != PARAM_T && param != PARAM_H {
                continue;
            }

            if let Some((first_surface, _)) = prod_def.fixed_surfaces() {
                if first_surface.surface_type == 100 {
                    // 等圧面
                    let pressure = first_surface.value() as i32;

                    // 初回のみ共通の格子座標・形状（メタデータ）を解析する
                    if grid_metadata.is_none() {
                        let (width, height) = submessage.grid_shape()?;
                        let mut lat_coords = Vec::with_capacity(height);
                        let mut lon_coords = Vec::with_capacity(width);

                        let mut latlons_iter = submessage.latlons()?;
                        for i in 0..(width * height) {
                            if let Some((lat, lon)) = latlons_iter.next() {
                                let lat_f = lat as f32;
                                let lon_f = (lon as f32).rem_euclid(360.0);
                                if i < width {
                                    lon_coords.push(lon_f);
                                }
                                if i % width == 0 {
                                    lat_coords.push(lat_f);
                                }
                            } else {
                                break;
                            }
                        }
                        grid_metadata = Some(GridMetadata {
                            lon_coords: Arc::new(lon_coords),
                            lat_coords: Arc::new(lat_coords),
                            width,
                            height,
                        });
                    }

                    let decoder = Grib2SubmessageDecoder::from(submessage)?;
                    decode_tasks.push((param, pressure, decoder));
                }
            }
        }

        let metadata = grid_metadata.as_ref().unwrap();

        let decoded: Vec<(GribParameter, i32, Vec<f32>)> = decode_tasks
            .into_par_iter()
            .map(|(param, pressure, decoder)| {
                let values: Vec<f32> = decoder.dispatch().unwrap().collect();
                (param, pressure, values)
            })
            .collect();

        let mut temp_storage: BTreeMap<i32, PressureLevelBuilder> = BTreeMap::new();
        for (param, pressure, values) in decoded {
            let grid = LatLonGrid {
                values,
                lon_coords: Arc::clone(&metadata.lon_coords),
                lat_coords: Arc::clone(&metadata.lat_coords),
                width: metadata.width,
                height: metadata.height,
            };

            let entry = temp_storage
                .entry(pressure)
                .or_insert_with(PressureLevelBuilder::default);
            match param {
                PARAM_U => entry.u_wind = Some(grid),
                PARAM_V => entry.v_wind = Some(grid),
                PARAM_T => entry.temp_k = Some(grid),
                PARAM_H => entry.height_gpm = Some(grid),
                _ => {}
            }
        }

        // 4つのデータが完全に揃っている気圧面だけを、完成したデータとして抽出
        let mut levels = BTreeMap::new();
        for (pressure, draft) in temp_storage {
            if let (Some(u_wind), Some(v_wind), Some(temp_k), Some(height_gpm)) =
                (draft.u_wind, draft.v_wind, draft.temp_k, draft.height_gpm)
            {
                levels.insert(
                    pressure,
                    AtmosphereLayer {
                        u_wind,
                        v_wind,
                        temp_k,
                        height_gpm,
                    },
                );
            }
        }

        Ok(Self { levels })
    }
}
