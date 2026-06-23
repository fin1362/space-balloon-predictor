use crate::simulation::Trajectory;

pub fn trajectory_to_kml(trajectory: &Trajectory) -> String {
    let mut ascent_coords = Vec::new();
    let mut descent_coords = Vec::new();

    for state in &trajectory.states {
        let coord_str = format!("{},{},{}", state.lon, state.lat, state.alt);

        if !state.is_burst {
            ascent_coords.push(coord_str);
        } else {
            descent_coords.push(coord_str);
        }
    }

    if let Some(last_ascent) = ascent_coords.last() {
        descent_coords.insert(0, last_ascent.clone());
    }

    let ascent_coordinates_str = ascent_coords.join("\n          ");
    let descent_coordinates_str = descent_coords.join("\n          ");

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
    </Placemark>
  </Document>
</kml>"#,
        ascent_coordinates_str, descent_coordinates_str
    )
}
