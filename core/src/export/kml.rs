use crate::geo::coords::EARTH_RADIUS;
use crate::engine::simulation::Trajectory;
use chrono::{DateTime, Duration, Utc};

fn format_elapsed(seconds: f64) -> String {
    let total = seconds as i64;
    let h = total / 3600;
    let m = (total % 3600) / 60;
    let s = total % 60;
    format!("{:02}:{:02}:{:02}", h, m, s)
}

fn compute_speed(trajectory: &Trajectory) -> Vec<f64> {
    let states = &trajectory.states;
    let n = states.len();
    let mut speeds = vec![0.0f64; n];

    for i in 0..n {
        let (prev, next) = if i == 0 {
            if n > 1 {
                (&states[0], &states[1])
            } else {
                (&states[0], &states[0])
            }
        } else if i == n - 1 {
            (&states[n - 2], &states[n - 1])
        } else {
            (&states[i - 1], &states[i + 1])
        };

        let dt = next.time - prev.time;
        if dt <= 0.0 {
            continue;
        }

        let dlat = (next.lat - prev.lat).to_radians();
        let dlon = (next.lon - prev.lon).to_radians();
        let avg_lat = ((prev.lat + next.lat) / 2.0).to_radians();

        let dx = EARTH_RADIUS * dlon * avg_lat.cos();
        let dy = EARTH_RADIUS * dlat;
        let dz = next.alt - prev.alt;

        let dist = (dx * dx + dy * dy + dz * dz).sqrt();
        speeds[i] = dist / dt;
    }

    speeds
}

pub fn trajectory_to_kml(trajectory: &Trajectory, launch_time: DateTime<Utc>) -> String {
    let speeds = compute_speed(trajectory);

    let mut ascent_coords = Vec::new();
    let mut descent_coords = Vec::new();
    let mut waypoints: Vec<(usize, f64, DateTime<Utc>, f64, f64, f64, bool)> = Vec::new();

    let mut last_wp_time = -60.0;

    for (i, state) in trajectory.states.iter().enumerate() {
        let abs_time = launch_time + Duration::milliseconds((state.time * 1000.0) as i64);
        let coord_str = format!("{},{},{}", state.lon, state.lat, state.alt);

        if !state.is_burst {
            ascent_coords.push(coord_str);
        } else {
            descent_coords.push(coord_str);
        }

        let is_burst = state.is_burst;
        let is_last = state.time >= trajectory.states.last().map_or(0.0, |s| s.time);

        let is_first_burst = is_burst && i > 0 && !trajectory.states[i - 1].is_burst;

        if state.time == 0.0 || is_first_burst || is_last || state.time - last_wp_time >= 60.0 {
            waypoints.push((
                i,
                state.time,
                abs_time,
                state.lat,
                state.lon,
                state.alt,
                is_first_burst,
            ));
            last_wp_time = state.time;
        }
    }

    if let Some(last_ascent) = ascent_coords.last() {
        descent_coords.insert(0, last_ascent.clone());
    }

    let ascent_coordinates_str = ascent_coords.join("\n          ");
    let descent_coordinates_str = descent_coords.join("\n          ");

    let mut waypoint_placemarks = String::new();
    for (idx, elapsed, abs_time, lat, lon, alt, _is_burst) in &waypoints {
        let elapsed_str = format_elapsed(*elapsed);
        let abs_str = abs_time.format("%Y-%m-%dT%H:%M:%SZ").to_string();
        let ns = if *lat >= 0.0 { "N" } else { "S" };
        let ew = if *lon >= 0.0 { "E" } else { "W" };
        let speed_kmh = speeds[*idx] * 3.6;

        waypoint_placemarks.push_str(&format!(
            r#"
    <Placemark>
      <description>時刻: {abs_str}
経過: {elapsed_str}
高度: {alt:.1} m
速度: {speed_kmh:.1} km/h
位置: {:.4}°{}, {:.4}°{}</description>
      <styleUrl>#waypoint_style</styleUrl>
      <Point>
        <altitudeMode>absolute</altitudeMode>
        <coordinates>{lon},{lat},{alt}</coordinates>
      </Point>
    </Placemark>"#,
            lat.abs(),
            ns,
            lon.abs(),
            ew,
            abs_str = abs_str,
            elapsed_str = elapsed_str,
            alt = alt,
            speed_kmh = speed_kmh,
            lon = lon,
            lat = lat,
        ));
    }

    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<kml xmlns="http://www.opengis.net/kml/2.2">
  <Document>
    <name>Space Balloon Flight Trajectory</name>
    <description>Simulated 4D space-time balloon flight path</description>

    <Style id="ascent_style">
      <LineStyle>
        <color>ff0000ff</color>
        <width>4</width>
      </LineStyle>
    </Style>

    <Style id="descent_style">
      <LineStyle>
        <color>ffff0000</color>
        <width>4</width>
      </LineStyle>
    </Style>

    <Style id="waypoint_style">
      <IconStyle>
        <color>ff00ff00</color>
        <scale>0.8</scale>
        <Icon>
          <href>http://maps.google.com/mapfiles/kml/shapes/placemark_circle.png</href>
        </Icon>
      </IconStyle>
    </Style>

    <Placemark>
      <name>Ascent Phase</name>
      <styleUrl>#ascent_style</styleUrl>
      <LineString>
        <extrude>0</extrude>
        <tessellate>1</tessellate>
        <altitudeMode>absolute</altitudeMode>
        <coordinates>
          {}
        </coordinates>
      </LineString>
    </Placemark>

    <Placemark>
      <name>Descent Phase</name>
      <styleUrl>#descent_style</styleUrl>
      <LineString>
        <extrude>0</extrude>
        <tessellate>1</tessellate>
        <altitudeMode>absolute</altitudeMode>
        <coordinates>
          {}
        </coordinates>
      </LineString>
    </Placemark>{}
  </Document>
</kml>"#,
        ascent_coordinates_str, descent_coordinates_str, waypoint_placemarks
    )
}
