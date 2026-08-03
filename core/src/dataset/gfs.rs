use chrono::{DateTime, Utc};

pub fn gfs_file_url(date_str: &str, cycle_str: &str, forecast_hour: u32) -> String {
    let filename = format!("gfs.t{}z.pgrb2full.0p50.f{:03}", cycle_str, forecast_hour);
    format!(
        "https://nomads.ncep.noaa.gov/pub/data/nccf/com/gfs/prod/gfs.{}/{}/atmos/{}",
        date_str, cycle_str, filename
    )
}

pub struct GfsForecast {
    /// ダウンロード先URL
    pub url: String,
    /// 予報の有効時刻
    pub time: DateTime<Utc>,
    /// 推奨ローカル保存パス
    pub local_path: String,
}

pub struct GfsForecastSet {
    /// 3時間間隔で発射時刻を挟む予報 (通常3件)
    pub forecasts: Vec<GfsForecast>,
    /// 最初の予報時刻からの発射オフセット (時間単位)
    pub launch_offset_hours: f64,
}

/// 発射時刻に基づき、必要なGFS予報のURL一覧を解決する
/// GFSは3時間ごとの予報を提供するため、発射時刻を挟む3時間区間の
/// 前後2件 + 中間1件 = 3件の予報URLを返す。
pub fn resolve_gfs_forecasts(
    gfs_run_time: DateTime<Utc>,
    launch_time: DateTime<Utc>,
) -> Result<GfsForecastSet, Box<dyn std::error::Error>> {
    let total_diff_seconds = launch_time
        .signed_duration_since(gfs_run_time)
        .num_seconds();
    if total_diff_seconds < 0 {
        return Err(
            "Error: Launch time cannot be before the GFS model initialization run time.".into(),
        );
    }

    let diff_hours = total_diff_seconds as f64 / 3600.0;
    let forecast_hour_low = ((diff_hours / 3.0).floor() as u32) * 3;
    let launch_offset_hours = diff_hours - (forecast_hour_low as f64);

    let date_str = gfs_run_time.format("%Y%m%d").to_string();
    let cycle_str = gfs_run_time.format("%H").to_string();

    let forecasts: Vec<GfsForecast> = [0u32, 3, 6]
        .iter()
        .map(|&offset| {
            let fh = forecast_hour_low + offset;
            let time = gfs_run_time + chrono::Duration::hours(fh as i64);
            GfsForecast {
                url: gfs_file_url(&date_str, &cycle_str, fh),
                time,
                local_path: format!("./gfs_{}_{}_f{:03}.grib2", date_str, cycle_str, fh),
            }
        })
        .collect();

    Ok(GfsForecastSet {
        forecasts,
        launch_offset_hours,
    })
}
