import { invoke } from "@tauri-apps/api/core"
import { save } from "@tauri-apps/plugin-dialog"
import type { MonteCarloPoint, TrajectoryPoint } from "@/types"

export interface TrajectoryKmlParams {
  ascent: TrajectoryPoint[]
  descent: TrajectoryPoint[]
  launchLat?: number
  launchLon?: number
  burstLat?: number
  burstLon?: number
  burstAlt?: number
  landingLat?: number
  landingLon?: number
  monteCarloPoints?: MonteCarloPoint[]
}

function escapeXml(value: string): string {
  return value
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&apos;")
}

function coordinates(path: TrajectoryPoint[]): string {
  return path
    .filter((p) => Number.isFinite(p.lon) && Number.isFinite(p.lat) && Number.isFinite(p.alt))
    .map((p) => `${p.lon},${p.lat},${p.alt}`)
    .join("\n")
}

const ASCENT_COLOR = "ff0000ff"
const DESCENT_COLOR = "ffff0000"

function linePlacemark(name: string, path: TrajectoryPoint[]): string {
  return `    <Placemark>
      <name>${escapeXml(name)}</name>
      <styleUrl>#${name}</styleUrl>
      <LineString>
        <tessellate>1</tessellate>
        <extrude>1</extrude>
        <altitudeMode>absolute</altitudeMode>
        <coordinates>
${coordinates(path)}
        </coordinates>
      </LineString>
    </Placemark>`
}

function pointPlacemark(name: string, lon: number, lat: number, alt: number | null | undefined): string {
  const altitude = alt == null || !Number.isFinite(alt) ? 0 : alt
  return `    <Placemark>
      <name>${escapeXml(name)}</name>
      <Point>
        <altitudeMode>absolute</altitudeMode>
        <coordinates>${lon},${lat},${altitude}</coordinates>
      </Point>
    </Placemark>`
}

export function buildTrajectoryKml({
  ascent,
  descent,
  launchLat,
  launchLon,
  burstLat,
  burstLon,
  burstAlt,
  landingLat,
  landingLon,
  monteCarloPoints,
}: TrajectoryKmlParams): string {
  const hasAscent = ascent.length > 0
  const hasDescent = descent.length > 0

  const lines: string[] = []
  if (hasAscent) lines.push(linePlacemark("ascent", ascent))
  if (hasDescent) lines.push(linePlacemark("descent", descent))

  const points: string[] = []
  if (
    launchLat != null &&
    launchLon != null &&
    Number.isFinite(launchLat) &&
    Number.isFinite(launchLon)
  ) {
    points.push(pointPlacemark("launch", launchLon, launchLat, 0))
  }
  if (
    burstLat != null &&
    burstLon != null &&
    Number.isFinite(burstLat) &&
    Number.isFinite(burstLon)
  ) {
    points.push(pointPlacemark("burst", burstLon, burstLat, burstAlt))
  }
  if (
    landingLat != null &&
    landingLon != null &&
    Number.isFinite(landingLat) &&
    Number.isFinite(landingLon)
  ) {
    points.push(pointPlacemark("landing", landingLon, landingLat, null))
  }
  if (monteCarloPoints && monteCarloPoints.length > 0) {
    const scatter = monteCarloPoints
      .filter((p) => Number.isFinite(p.landing_lon) && Number.isFinite(p.landing_lat))
      .map((p) => pointPlacemark("mc-point", p.landing_lon, p.landing_lat, null))
      .join("\n")
    points.push(`  <Folder>
      <name>Monte Carlo Landings</name>
${scatter}
    </Folder>`)
  }

  return `<?xml version="1.0" encoding="UTF-8"?>
<kml xmlns="http://www.opengis.net/kml/2.2">
  <Document>
    <name>Balloon Trajectory</name>
    <Style id="ascent">
      <LineStyle>
        <color>${ASCENT_COLOR}</color>
        <width>3</width>
      </LineStyle>
    </Style>
    <Style id="descent">
      <LineStyle>
        <color>${DESCENT_COLOR}</color>
        <width>3</width>
      </LineStyle>
    </Style>
${lines.join("\n")}
${points.join("\n")}
  </Document>
</kml>`
}

export async function saveKmlFile(
  filename: string,
  kmlString: string,
): Promise<string | null> {
  const path = await save({
    defaultPath: filename,
    filters: [{ name: "KML", extensions: ["kml"] }],
  })
  if (!path) return null
  await invoke("save_kml", { path, contents: kmlString })
  return path
}