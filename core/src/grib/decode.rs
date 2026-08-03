use std::collections::BTreeMap;

use crate::geo::grid::LatLonGrid;

use super::parameter::{GribParameter, PARAM_H, PARAM_T, PARAM_U, PARAM_V};
use super::types::{AtmosphereLayer, PressureLevelBuilder};

/// GRIBパラメータをPressureLevelBuilderの対応フィールドに格納
pub(crate) fn store_grib2_param(
    builder: &mut PressureLevelBuilder,
    param: GribParameter,
    grid: LatLonGrid,
) {
    match param {
        PARAM_U => builder.u_wind = Some(grid),
        PARAM_V => builder.v_wind = Some(grid),
        PARAM_T => builder.temp_k = Some(grid),
        PARAM_H => builder.height_gpm = Some(grid),
        _ => {}
    }
}

/// PressureLevelBuilderから、4変数が揃った等圧面だけをAtmosphereに変換
pub(crate) fn build_levels(
    temp_storage: BTreeMap<i32, PressureLevelBuilder>,
) -> BTreeMap<i32, AtmosphereLayer> {
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
    levels
}
