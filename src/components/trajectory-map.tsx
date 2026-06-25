import { Map, Marker, useMap, useControl } from "@vis.gl/react-maplibre"
import { MapboxOverlay } from "@deck.gl/mapbox"
import { PathLayer, ScatterplotLayer } from "@deck.gl/layers"
import maplibregl from "maplibre-gl"
import { useMemo, useEffect, useCallback } from "react"
import "maplibre-gl/dist/maplibre-gl.css"

interface TrajectoryPoint {
  lat: number
  lon: number
  alt: number
}

interface MonteCarloPoint {
  landing_lat: number
  landing_lon: number
  burst_altitude: number
  deviation_sigma: number
}

interface MonteCarloTrajectory {
  ascent_path: TrajectoryPoint[]
  descent_path: TrajectoryPoint[]
}

interface TrajectoryMapProps {
  predictionData: any
  monteCarloData?: {
    points: MonteCarloPoint[]
    mean_landing_lat: number
    mean_landing_lon: number
    mean_ascent_path: TrajectoryPoint[]
    mean_descent_path: TrajectoryPoint[]
    trajectories: MonteCarloTrajectory[]
  } | null
  selectedPointIndex?: number | null
  onPointSelect?: (index: number | null) => void
  launchLat?: number
  launchLon?: number
  mapSelectionMode?: boolean
  onMapClick?: (lat: number, lon: number) => void
}

function DeckGLOverlay(props: { layers: any[]; interleaved?: boolean }) {
  const overlay = useControl<MapboxOverlay>(() => new MapboxOverlay(props))
  overlay.setProps(props)
  return null
}

function MapController({
  trajectoryPoints,
  monteCarloPoints,
}: {
  trajectoryPoints: TrajectoryPoint[] | null
  monteCarloPoints?: MonteCarloPoint[] | null
}) {
  const { current: map } = useMap()

  useEffect(() => {
    if (!map) return

    if (monteCarloPoints && monteCarloPoints.length > 0) {
      const coords: [number, number][] = monteCarloPoints.map((p) => [
        p.landing_lon,
        p.landing_lat,
      ])
      if (coords.length === 0) return

      const bounds = coords.reduce(
        (b, c) => b.extend(c),
        new maplibregl.LngLatBounds(coords[0], coords[0]),
      )
      map.fitBounds(bounds, { padding: 60, duration: 1200 })
    } else if (trajectoryPoints && trajectoryPoints.length > 0) {
      const coords: [number, number][] = trajectoryPoints.map((p) => [
        p.lon,
        p.lat,
      ])
      if (coords.length === 0) return

      const bounds = coords.reduce(
        (b, c) => b.extend(c),
        new maplibregl.LngLatBounds(coords[0], coords[0]),
      )
      map.fitBounds(bounds, { padding: 60, duration: 1200 })
    }
  }, [map, trajectoryPoints, monteCarloPoints])

  return null
}

function deviationToColor(deviation: number): [number, number, number, number] {
  const absDev = Math.abs(deviation)
  if (absDev <= 1) return [34, 197, 94, 200]    // green
  if (absDev <= 2) return [234, 179, 8, 200]    // yellow
  return [239, 68, 68, 200]                      // red
}

export function TrajectoryMap({
  predictionData,
  monteCarloData = null,
  selectedPointIndex = null,
  onPointSelect,
  launchLat,
  launchLon,
  mapSelectionMode = false,
  onMapClick,
}: TrajectoryMapProps) {
  const handleMapClick = useCallback((e: any) => {
    if (!mapSelectionMode || !onMapClick) return
    const { lat, lng } = e.lngLat
    onMapClick(lat, lng)
  }, [mapSelectionMode, onMapClick])

  const hasLaunchPos =
    launchLat != null && launchLon != null &&
    !isNaN(launchLat) && !isNaN(launchLon) &&
    launchLat >= -90 && launchLat <= 90 &&
    launchLon >= -180 && launchLon <= 180

  const allPoints = useMemo(() => {
    if (monteCarloData) {
      const ascent: TrajectoryPoint[] = monteCarloData.mean_ascent_path ?? []
      const descent: TrajectoryPoint[] = monteCarloData.mean_descent_path ?? []
      return [...ascent, ...descent.slice(1)]
    }
    if (!predictionData) return null
    const ascent: TrajectoryPoint[] = predictionData.ascent_path ?? []
    const descent: TrajectoryPoint[] = predictionData.descent_path ?? []
    return [...ascent, ...descent.slice(1)]
  }, [predictionData, monteCarloData])

  const deckLayers = useMemo(() => {
    const layers: any[] = []

    if (monteCarloData && monteCarloData.points.length > 0) {
      // 選択中のサンプルの経路
      if (selectedPointIndex !== null && selectedPointIndex !== undefined) {
        const traj = monteCarloData.trajectories[selectedPointIndex]
        if (traj) {
          const selAscent: [number, number, number][] = (traj.ascent_path ?? []).map((p) => [p.lon, p.lat, p.alt])
          const selDescent: [number, number, number][] = (traj.descent_path ?? []).map((p) => [p.lon, p.lat, p.alt])

          if (selAscent.length >= 2) {
            layers.push(
              new PathLayer({
                id: "mc-selected-ascent",
                data: [{ path: selAscent }],
                getPath: (d: { path: [number, number, number][] }) => d.path,
                getColor: [255, 100, 100, 240],
                widthUnits: "pixels",
                getWidth: 5,
                billboard: true,
                rounded: true,
                jointRounded: true,
              }),
            )
          }

          if (selDescent.length >= 2) {
            layers.push(
              new PathLayer({
                id: "mc-selected-descent",
                data: [{ path: selDescent }],
                getPath: (d: { path: [number, number, number][] }) => d.path,
                getColor: [100, 160, 255, 240],
                widthUnits: "pixels",
                getWidth: 5,
                billboard: true,
                rounded: true,
                jointRounded: true,
              }),
            )
          }
        }
      }

      // 平均経路（上昇・落下）
      const meanAscent: [number, number, number][] = (
        monteCarloData.mean_ascent_path ?? []
      ).map((p: TrajectoryPoint) => [p.lon, p.lat, p.alt])
      const meanDescent: [number, number, number][] = (
        monteCarloData.mean_descent_path ?? []
      ).map((p: TrajectoryPoint) => [p.lon, p.lat, p.alt])

      if (meanAscent.length >= 2) {
        layers.push(
          new PathLayer({
            id: "mc-mean-ascent",
            data: [{ path: meanAscent }],
            getPath: (d: { path: [number, number, number][] }) => d.path,
            getColor: [239, 68, 68, 180],
            widthUnits: "pixels",
            getWidth: 3,
            billboard: true,
            rounded: true,
            jointRounded: true,
          }),
        )
      }

      if (meanDescent.length >= 2) {
        layers.push(
          new PathLayer({
            id: "mc-mean-descent",
            data: [{ path: meanDescent }],
            getPath: (d: { path: [number, number, number][] }) => d.path,
            getColor: [59, 130, 246, 180],
            widthUnits: "pixels",
            getWidth: 3,
            billboard: true,
            rounded: true,
            jointRounded: true,
          }),
        )
      }

      // 散布点（sigma > 0 の場合のみ）
      if (monteCarloData.points.length > 1) {
        layers.push(
          new ScatterplotLayer({
            id: "monte-carlo-landing-points",
            data: monteCarloData.points,
            getPosition: (d: MonteCarloPoint) => [d.landing_lon, d.landing_lat],
            getFillColor: (_d: MonteCarloPoint, { index }: { index: number }) =>
              selectedPointIndex === index
                ? [255, 255, 255, 255]
                : deviationToColor(_d.deviation_sigma),
            getLineColor: (_d: MonteCarloPoint, { index }: { index: number }) =>
              selectedPointIndex === index
                ? [0, 0, 0, 255]
                : [255, 255, 255, 220],
            getRadius: (_d: MonteCarloPoint, { index }: { index: number }) =>
              selectedPointIndex === index ? 120 : 80,
            radiusUnits: "meters",
            radiusMinPixels: 4,
            radiusMaxPixels: 12,
            stroked: true,
            lineWidthMinPixels: selectedPointIndex !== null ? 2 : 1,
            pickable: true,
            onClick: (info: any) => {
              if (info.object) {
                const idx = monteCarloData.points.indexOf(info.object)
                onPointSelect?.(idx === selectedPointIndex ? null : idx)
              }
            },
          }),
        )
      }

      // 平均着地点
      if (
        monteCarloData.mean_landing_lat !== undefined &&
        monteCarloData.mean_landing_lon !== undefined
      ) {
        layers.push(
          new ScatterplotLayer({
            id: "monte-carlo-mean-point",
            data: [{ lat: monteCarloData.mean_landing_lat, lon: monteCarloData.mean_landing_lon }],
            getPosition: (d: { lat: number; lon: number }) => [d.lon, d.lat],
            getFillColor: [255, 255, 255, 255],
            getRadius: 120,
            radiusUnits: "meters",
            radiusMinPixels: 6,
            radiusMaxPixels: 16,
            stroked: true,
            getLineColor: [0, 0, 0, 255],
            lineWidthMinPixels: 2,
          }),
        )
      }
    } else if (predictionData) {
      const ascentPath: [number, number, number][] = (
        predictionData.ascent_path ?? []
      ).map((p: TrajectoryPoint) => [p.lon, p.lat, p.alt])
      const descentPath: [number, number, number][] = (
        predictionData.descent_path ?? []
      ).map((p: TrajectoryPoint) => [p.lon, p.lat, p.alt])

      if (ascentPath.length >= 2) {
        layers.push(
          new PathLayer({
            id: "ascent-line-3d",
            data: [{ path: ascentPath }],
            getPath: (d: { path: [number, number, number][] }) => d.path,
            getColor: [239, 68, 68, 220],
            widthUnits: "pixels",
            getWidth: 4,
            billboard: true,
            rounded: true,
            jointRounded: true,
          }),
        )
      }

      if (descentPath.length >= 2) {
        layers.push(
          new PathLayer({
            id: "descent-line-3d",
            data: [{ path: descentPath }],
            getPath: (d: { path: [number, number, number][] }) => d.path,
            getColor: [59, 130, 246, 220],
            widthUnits: "pixels",
            getWidth: 4,
            billboard: true,
            rounded: true,
            jointRounded: true,
          }),
        )
      }
    }

    return layers
  }, [predictionData, monteCarloData, selectedPointIndex])

  const landingPos = useMemo(() => {
    if (monteCarloData) return null
    if (!predictionData) return null
    return { lat: predictionData.landing_lat, lon: predictionData.landing_lon }
  }, [predictionData, monteCarloData])

  const monteCarloPoints = useMemo(() => {
    return monteCarloData?.points ?? null
  }, [monteCarloData])

  return (
    <div className={`w-full h-full ${mapSelectionMode ? "cursor-crosshair" : ""}`}>
      <Map
        initialViewState={{
          longitude: 135,
          latitude: 35,
          zoom: 5,
        }}
        style={{ width: "100%", height: "100%" }}
        mapStyle="https://tiles.openfreemap.org/styles/liberty"
        onClick={handleMapClick}
      >
        <MapController trajectoryPoints={allPoints} monteCarloPoints={monteCarloPoints} />

        {deckLayers.length > 0 && (
          <DeckGLOverlay layers={deckLayers} interleaved={true} />
        )}

        {hasLaunchPos && (
          <Marker
            longitude={launchLon!}
            latitude={launchLat!}
            anchor="bottom"
          >
            <div className="relative">
              <div className="w-3 h-3 bg-red-500 border-2 border-white rounded-full shadow-lg" />
              <span className="absolute -top-5 left-1/2 -translate-x-1/2 text-[10px] font-bold bg-background/80 px-1 rounded whitespace-nowrap">
                放球
              </span>
            </div>
          </Marker>
        )}

        {landingPos && (
          <Marker
            longitude={landingPos.lon}
            latitude={landingPos.lat}
            anchor="bottom"
          >
            <div className="relative">
              <div className="w-3 h-3 bg-blue-500 border-2 border-white rounded-full shadow-lg" />
              <span className="absolute -top-5 left-1/2 -translate-x-1/2 text-[10px] font-bold bg-background/80 px-1 rounded whitespace-nowrap">
                着地
              </span>
            </div>
          </Marker>
        )}
      </Map>

      {monteCarloData && monteCarloData.points.length > 0 && (
        <div className="absolute bottom-4 right-4 z-10 bg-background/90 border rounded-lg p-3 text-xs space-y-1.5">
          <p className="font-medium text-[11px] mb-1">偏差 (σ)</p>
          <div className="flex items-center gap-2">
            <span className="w-3 h-3 rounded-full bg-[rgb(34,197,94)]" />
            <span>±1σ 以内</span>
          </div>
          <div className="flex items-center gap-2">
            <span className="w-3 h-3 rounded-full bg-[rgb(234,179,8)]" />
            <span>±1σ〜2σ</span>
          </div>
          <div className="flex items-center gap-2">
            <span className="w-3 h-3 rounded-full bg-[rgb(239,68,68)]" />
            <span>±2σ 超</span>
          </div>
          <div className="flex items-center gap-2">
            <span className="w-3 h-3 rounded-full bg-white border-2 border-black" />
            <span>平均着地点</span>
          </div>
        </div>
      )}
    </div>
  )
}