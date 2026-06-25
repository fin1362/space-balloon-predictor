mod coords;
mod grib_reader;
mod grid;
mod interpolation;
mod kml;
mod physics;
mod simulation;

use chrono::{DateTime, Timelike, Utc};
use serde::Serialize;
use std::fs::File;
use std::io::copy;
use std::path::Path;

use coords::{EARTH_RADIUS, Geodetic};
use grib_reader::Atmosphere;
use simulation::{SimConfig, Simulator, Trajectory};

use rand::Rng;
use rand_distr::Normal;
use rayon::prelude::*;

#[derive(Serialize)]
struct TrajectoryPoint {
    lat: f64,
    lon: f64,
    alt: f64,
}

#[derive(Serialize)]
struct SimulationResult {
    ascent_path: Vec<TrajectoryPoint>,
    descent_path: Vec<TrajectoryPoint>,
    stratosphere_duration_s: f64,
    max_altitude: f64,
    landing_lat: f64,
    landing_lon: f64,
    drift_km: f64,
    total_duration_s: f64,
}

#[derive(Serialize)]
struct MonteCarloPoint {
    landing_lat: f64,
    landing_lon: f64,
    burst_altitude: f64,
    deviation_sigma: f64,
}

#[derive(Serialize)]
struct MonteCarloTrajectory {
    ascent_path: Vec<TrajectoryPoint>,
    descent_path: Vec<TrajectoryPoint>,
}

#[derive(Serialize)]
struct MonteCarloResult {
    points: Vec<MonteCarloPoint>,
    mean_landing_lat: f64,
    mean_landing_lon: f64,
    mean_ascent_path: Vec<TrajectoryPoint>,
    mean_descent_path: Vec<TrajectoryPoint>,
    trajectories: Vec<MonteCarloTrajectory>,
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
    let cycle_hour = (gfs_run_time.hour() / 6) * 6;
    let rounded_gfs_time = gfs_run_time
        .date_naive()
        .and_hms_opt(cycle_hour, 0, 0)
        .unwrap()
        .and_utc();

    let total_diff_seconds = launch_time
        .signed_duration_since(rounded_gfs_time)
        .num_seconds();
    if total_diff_seconds < 0 {
        return Err(
            "Error: Launch time cannot be before the GFS model initialization run time.".into(),
        );
    }

    let diff_hours = total_diff_seconds as f64 / 3600.0;
    let forecast_hour_low = ((diff_hours / 3.0).floor() as u32) * 3;
    let launch_offset_hours = diff_hours - (forecast_hour_low as f64);

    let date_str = rounded_gfs_time.format("%Y%m%d").to_string();
    let cycle_str = rounded_gfs_time.format("%H").to_string();

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
    let ((r1, r2), r3) = rayon::join(
        || rayon::join(
            || Atmosphere::new(&path_low).map_err(|e| e.to_string()),
            || Atmosphere::new(&path_mid).map_err(|e| e.to_string()),
        ),
        || Atmosphere::new(&path_high).map_err(|e| e.to_string()),
    );
    let (env_earliest, env_middle, env_latest) = (r1?, r2?, r3?);

    Ok((
        env_earliest,
        env_middle,
        env_latest,
        launch_offset_hours,
    ))
}

fn trajectory_to_result(trajectory: &Trajectory, launch_site: Geodetic) -> SimulationResult {
    let burst_idx = trajectory
        .states
        .iter()
        .position(|s| s.is_burst)
        .unwrap_or(trajectory.states.len());

    let ascent_path: Vec<TrajectoryPoint> = trajectory.states[..=burst_idx.min(trajectory.states.len() - 1)]
        .iter()
        .map(|s| TrajectoryPoint {
            lat: s.lat,
            lon: s.lon,
            alt: s.alt,
        })
        .collect();

    let descent_path: Vec<TrajectoryPoint> = if burst_idx < trajectory.states.len() {
        trajectory.states[burst_idx..]
            .iter()
            .map(|s| TrajectoryPoint {
                lat: s.lat,
                lon: s.lon,
                alt: s.alt,
            })
            .collect()
    } else {
        Vec::new()
    };

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
        ascent_path,
        descent_path,
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
async fn run_monte_carlo(
    launch_lat: f64,
    launch_lon: f64,
    launch_alt: f64,
    gfs_run_time: String,
    launch_time: String,
    ascent_rate: f64,
    descent_rate: f64,
    burst_altitude_mean: f64,
    burst_altitude_std: f64,
    num_samples: u32,
) -> Result<MonteCarloResult, String> {
    let gfs_run: DateTime<Utc> = gfs_run_time.parse().map_err(|e| format!("Invalid gfs_run_time: {}", e))?;
    let launch: DateTime<Utc> = launch_time.parse().map_err(|e| format!("Invalid launch_time: {}", e))?;

    let launch_site = Geodetic {
        lat: launch_lat,
        lon: launch_lon,
        alt: launch_alt,
    };

    tokio::task::spawn_blocking(move || -> Result<MonteCarloResult, String> {
        let work_dir = Path::new(".").to_path_buf();

        let (env_earliest, env_middle, env_latest, launch_offset_hours) =
            download_gfs_series(&work_dir, gfs_run, launch)?;

        let use_scatter = burst_altitude_std > 0.0;

        let sample_count = if use_scatter { num_samples } else { 1 };

        // サンプルするバースト高度を事前に生成
        let burst_altitudes: Vec<f64> = if use_scatter {
            let normal = Normal::new(burst_altitude_mean, burst_altitude_std)
                .map_err(|e| format!("Invalid distribution parameters: {}", e))?;
            let mut rng = rand::thread_rng();
            (0..sample_count)
                .map(|_| rng.sample(normal).max(0.0))
                .collect()
        } else {
            vec![burst_altitude_mean]
        };

        // Rayon で並列シミュレーション
        let (points, trajectories): (Vec<MonteCarloPoint>, Vec<MonteCarloTrajectory>) = burst_altitudes
            .par_iter()
            .map(|&sampled_burst| {
                let deviation = if use_scatter {
                    (sampled_burst - burst_altitude_mean) / burst_altitude_std
                } else {
                    0.0
                };

                let config = SimConfig {
                    launch_site,
                    ascent_rate_m_s: ascent_rate,
                    ground_descend_rate_m_s: descent_rate,
                    burst_altitude_m: sampled_burst,
                    dt: 5.0,
                };

                let simulator = Simulator::new(
                    config,
                    env_earliest.clone(),
                    env_middle.clone(),
                    env_latest.clone(),
                    launch_offset_hours,
                );
                let trajectory = simulator.run();
                let result = trajectory_to_result(&trajectory, launch_site);

                let mc_point = MonteCarloPoint {
                    landing_lat: result.landing_lat,
                    landing_lon: result.landing_lon,
                    burst_altitude: sampled_burst,
                    deviation_sigma: deviation,
                };

                let mc_traj = MonteCarloTrajectory {
                    ascent_path: result.ascent_path,
                    descent_path: result.descent_path,
                };

                (mc_point, mc_traj)
            })
            .unzip();

        let sum_lat: f64 = points.iter().map(|p| p.landing_lat).sum();
        let sum_lon: f64 = points.iter().map(|p| p.landing_lon).sum();

        // 平均バースト高度の経路を計算
        let mean_config = SimConfig {
            launch_site,
            ascent_rate_m_s: ascent_rate,
            ground_descend_rate_m_s: descent_rate,
            burst_altitude_m: burst_altitude_mean,
            dt: 5.0,
        };
        let mean_sim = Simulator::new(
            mean_config,
            env_earliest,
            env_middle,
            env_latest,
            launch_offset_hours,
        );
        let mean_trajectory = mean_sim.run();
        let mean_result = trajectory_to_result(&mean_trajectory, launch_site);

        let n = sample_count as f64;
        Ok(MonteCarloResult {
            points,
            mean_landing_lat: if use_scatter { sum_lat / n } else { mean_result.landing_lat },
            mean_landing_lon: if use_scatter { sum_lon / n } else { mean_result.landing_lon },
            mean_ascent_path: mean_result.ascent_path,
            mean_descent_path: mean_result.descent_path,
            trajectories,
        })
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
        .invoke_handler(tauri::generate_handler![greet, run_simulation, run_monte_carlo])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
