use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::geo::grid::LatLonGrid;

use super::decode::{build_levels, store_grib2_param};
use super::parameter::GribParameter;
use super::types::{GridMetadata, PressureLevelBuilder, PressureUnit};
use super::Atmosphere;

pub struct Grib2File {
    pub(crate) path: PathBuf,
}

impl Grib2File {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, Box<dyn std::error::Error>> {
        let path = path.as_ref().to_path_buf();
        if !path.exists() {
            return Err(format!("File not found: {}", path.display()).into());
        }
        Ok(Self { path })
    }
}

struct Grib2Message<'a> {
    msg: &'a grib_reader::Message<'a>,
}

impl<'a> Grib2Message<'a> {
    fn new(msg: &'a grib_reader::Message<'a>) -> Self {
        Self { msg }
    }

    fn param(&self) -> Option<GribParameter> {
        let meta = self.msg.metadata();
        let discipline = meta.discipline?;
        let category = meta.parameter.category?;
        let number = meta.parameter.number;
        let param = GribParameter {
            discipline,
            category,
            number,
        };
        param.is_supported().then_some(param)
    }

    fn valid_time(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        let valid = self.msg.valid_time()?;
        let naive =
            chrono::NaiveDate::from_ymd_opt(valid.year as i32, valid.month as u32, valid.day as u32)
                .and_then(|d| {
                    d.and_hms_opt(valid.hour as u32, valid.minute as u32, valid.second as u32)
                })?;
        Some(chrono::DateTime::from_naive_utc_and_offset(
            naive,
            chrono::Utc,
        ))
    }

    fn pressure_pa(&self, unit: PressureUnit) -> Option<i32> {
        let pd = self.msg.product_definition()?;
        let surface = pd.first_surface()?;
        if surface.surface_type != 100 {
            return None;
        }
        Some(unit.to_pa(surface.scaled_value_f64()) as i32)
    }

    fn grid_metadata(&self) -> Option<GridMetadata> {
        let (width, height) = self.msg.grid_shape();
        let lats = self.msg.latitudes().ok()??;
        let lons = self.msg.longitudes().ok()??;
        let lat_coords: Vec<f32> = lats.iter().map(|&v| v as f32).collect();
        let lon_coords: Vec<f32> = lons.iter().map(|&v| (v as f32).rem_euclid(360.0)).collect();
        Some(GridMetadata {
            lon_coords: Arc::new(lon_coords),
            lat_coords: Arc::new(lat_coords),
            width,
            height,
        })
    }

    fn decode_grid(&self, metadata: &GridMetadata) -> Option<LatLonGrid> {
        let values = self.msg.read_flat_data_as_f32().ok()?;
        Some(LatLonGrid {
            values,
            lon_coords: Arc::clone(&metadata.lon_coords),
            lat_coords: Arc::clone(&metadata.lat_coords),
            width: metadata.width,
            height: metadata.height,
        })
    }
}

impl Grib2File {
    pub fn to_atmosphere(
        &self,
        unit: PressureUnit,
    ) -> Result<Vec<(chrono::DateTime<chrono::Utc>, Atmosphere)>, Box<dyn std::error::Error>> {
        let file = grib_reader::GribFile::open(&self.path)?;

        let mut time_builders: BTreeMap<
            chrono::DateTime<chrono::Utc>,
            (BTreeMap<i32, PressureLevelBuilder>, Option<GridMetadata>),
        > = BTreeMap::new();

        for msg in file.messages() {
            let grib = Grib2Message::new(&msg);
            let param = match grib.param() {
                Some(p) => p,
                _ => continue,
            };
            let valid_time = match grib.valid_time() {
                Some(t) => t,
                None => continue,
            };
            let pressure_pa = match grib.pressure_pa(unit) {
                Some(p) => p,
                None => continue,
            };

            let (temp_storage, grid_metadata) = time_builders
                .entry(valid_time)
                .or_insert_with(|| (BTreeMap::new(), None));

            if grid_metadata.is_none() {
                *grid_metadata = grib.grid_metadata();
            }
            let metadata = grid_metadata.as_ref().unwrap();
            let grid = match grib.decode_grid(metadata) {
                Some(g) => g,
                None => continue,
            };
            let entry = temp_storage.entry(pressure_pa).or_default();
            store_grib2_param(entry, param, grid);
        }

        let mut result: Vec<(chrono::DateTime<chrono::Utc>, Atmosphere)> = Vec::new();
        for (valid_time, (temp_storage, _)) in time_builders {
            let levels = build_levels(temp_storage);
            if !levels.is_empty() {
                result.push((
                    valid_time,
                    Atmosphere {
                        levels,
                        secondary_key: None,
                        secondary_levels: HashSet::new(),
                    },
                ));
            }
        }

        result.sort_by_key(|(dt, _)| *dt);
        Ok(result)
    }
}
