mod coords;
mod grib_reader;
mod grid;
mod interpolation;
mod kml;
mod physics;
mod simulation;

use chrono::{DateTime, Utc};
use serde::Serialize;
use std::fs::File;
use std::io::copy;
use std::path::Path;

use coords::{EARTH_RADIUS, Geodetic};
use grib_reader::Atmosphere;
use simulation::{SimConfig, Simulator, Trajectory};

#[derive(Serialize)]
struct BalloonStateInfo {
    lat: f64,
    lon: f64,
    alt: f64,
    time: f64,
    is_burst: bool,
}

#[derive(Serialize)]
struct SimulationResult {
    states: Vec<BalloonStateInfo>,
    stratosphere_duration_s: f64,
    max_altitude: f64,
    landing_lat: f64,
    landing_lon: f64,
    drift_km: f64,
    total_duration_s: f64,
}

fn download_gfs_file(
    work_dir: &Path,
    date_str: &str,
    cycle_str: &str,
    forecast_hour: u32,
) -> Result<String, String> {
    let filename = format!("gfs.t{}z.pgrb2full.0p50.f{:03}", cycle_str, forecast_hour);
    let local_path = work_dir.join(format!(
        "gfs_{}_{}_f{:03}.grib2",
        date_str, cycle_str, forecast_hour
    ));

    if local_path.exists() {
        println!(
            "  File '{}' already exists locally. Skipping download.",
            local_path.display()
        );
        return Ok(local_path.to_string_lossy().into_owned());
    }

    let url = format!(
        "https://nomads.ncep.noaa.gov/pub/data/nccf/com/gfs/prod/gfs.{}/{}/atmos/{}",
        date_str, cycle_str, filename
    );

    println!("  Downloading '{}' from NOAA NOMADS...", url);
    let mut response = reqwest::blocking::get(&url).map_err(|e| e.to_string())?;
    if !response.status().is_success() {
        return Err(format!(
            "Failed to download GRIB file. HTTP Status: {}.\n\
             (Note: NOAA only stores the last 10 days of forecast data. Older dates will result in errors.)",
            response.status()
        ));
    }

    let mut dest = File::create(&local_path).map_err(|e| e.to_string())?;
    copy(&mut response, &mut dest).map_err(|e| e.to_string())?;
    println!("  Saved successfully to '{}'.", local_path.display());

    Ok(local_path.to_string_lossy().into_owned())
}

fn download_gfs_series(
    work_dir: &Path,
    gfs_run_time: DateTime<Utc>,
    launch_time: DateTime<Utc>,
) -> Result<(Atmosphere, Atmosphere, Atmosphere, f64), String> {
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

    println!(
        "Downloading GFS (f{:03}→f{:03}→f{:03}), offset {:.2}h",
        forecast_hour_low,
        forecast_hour_low + 3,
        forecast_hour_low + 6,
        launch_offset_hours
    );

    let path_low = download_gfs_file(work_dir, &date_str, &cycle_str, forecast_hour_low)?;
    let path_mid = download_gfs_file(work_dir, &date_str, &cycle_str, forecast_hour_low + 3)?;
    let path_high = download_gfs_file(work_dir, &date_str, &cycle_str, forecast_hour_low + 6)?;

    println!("Parsing GRIB2 files...");
    let env_earliest = Atmosphere::new(&path_low).map_err(|e| e.to_string())?;
    let env_middle = Atmosphere::new(&path_mid).map_err(|e| e.to_string())?;
    let env_latest = Atmosphere::new(&path_high).map_err(|e| e.to_string())?;

    Ok((
        env_earliest,
        env_middle,
        env_latest,
        launch_offset_hours,
    ))
}

fn trajectory_to_result(trajectory: &Trajectory, launch_site: Geodetic) -> SimulationResult {
    let states: Vec<BalloonStateInfo> = trajectory
        .states
        .iter()
        .map(|s| BalloonStateInfo {
            lat: s.lat,
            lon: s.lon,
            alt: s.alt,
            time: s.time,
            is_burst: s.is_burst,
        })
        .collect();

    let max_altitude = trajectory
        .states
        .iter()
        .map(|s| s.alt)
        .fold(0.0, f64::max);

    let stratosphere_duration_s = trajectory.stratosphere_duration_s();

    let (landing_lat, landing_lon, total_duration_s, drift_km) =
        if let Some(last) = trajectory.states.last() {
            let d_lat = (last.lat - launch_site.lat).to_radians();
            let d_lon = (last.lon - launch_site.lon).to_radians();
            let lat_avg = ((last.lat + launch_site.lat) / 2.0).to_radians();
            let drift = ((d_lat * EARTH_RADIUS).powi(2)
                + (d_lon * EARTH_RADIUS * lat_avg.cos()).powi(2))
                .sqrt()
                / 1000.0;
            (last.lat, last.lon, last.time, drift)
        } else {
            (0.0, 0.0, 0.0, 0.0)
        };

    SimulationResult {
        states,
        stratosphere_duration_s,
        max_altitude,
        landing_lat,
        landing_lon,
        drift_km,
        total_duration_s,
    }
}

#[tauri::command]
async fn run_simulation(
    launch_lat: f64,
    launch_lon: f64,
    launch_alt: f64,
    gfs_run_time: String,
    launch_time: String,
    ascent_rate: f64,
    descent_rate: f64,
    burst_altitude: f64,
) -> Result<SimulationResult, String> {
    let gfs_run: DateTime<Utc> = gfs_run_time.parse().map_err(|e| format!("Invalid gfs_run_time: {}", e))?;
    let launch: DateTime<Utc> = launch_time.parse().map_err(|e| format!("Invalid launch_time: {}", e))?;

    let launch_site = Geodetic {
        lat: launch_lat,
        lon: launch_lon,
        alt: launch_alt,
    };

    tokio::task::spawn_blocking(move || -> Result<SimulationResult, String> {
        let work_dir = Path::new(".").to_path_buf();

        let (env_earliest, env_middle, env_latest, launch_offset_hours) =
            download_gfs_series(&work_dir, gfs_run, launch)?;

        let config = SimConfig {
            launch_site,
            ascent_rate_m_s: ascent_rate,
            ground_descend_rate_m_s: descent_rate,
            burst_altitude_m: burst_altitude,
            dt: 5.0,
        };

        println!("Running simulation...");
        let simulator = Simulator::new(
            config,
            env_earliest,
            env_middle,
            env_latest,
            launch_offset_hours,
        );
        let trajectory = simulator.run();

        Ok(trajectory_to_result(&trajectory, launch_site))
    })
    .await
    .map_err(|e| format!("Task failed: {}", e))?
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet, run_simulation])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
