import { Map, Marker, useMap, useControl } from '@vis.gl/react-maplibre';
import { MapboxOverlay } from '@deck.gl/mapbox';
import { PathLayer } from '@deck.gl/layers';
import maplibregl from 'maplibre-gl';
import { useMemo, useEffect } from "react";
import 'maplibre-gl/dist/maplibre-gl.css';

interface TrajectoryPoint {
  lat: number;
  lon: number;
  alt: number;
}

interface TrajectoryMapProps {
  predictionData: any;
}

function DeckGLOverlay(props: { layers: any[]; interleaved?: boolean }) {
  const overlay = useControl<MapboxOverlay>(() => new MapboxOverlay(props));
  overlay.setProps(props);
  return null;
}

function MapController({ trajectoryPoints }: { trajectoryPoints: TrajectoryPoint[] | null }) {
  const { current: map } = useMap();

  useEffect(() => {
    if (!map || !trajectoryPoints || trajectoryPoints.length === 0) return;

    const coords: [number, number][] = trajectoryPoints.map((p) => [p.lon, p.lat]);
    if (coords.length === 0) return;

    const bounds = coords.reduce(
      (b, c) => b.extend(c),
      new maplibregl.LngLatBounds(coords[0], coords[0])
    );
    map.fitBounds(bounds, { padding: 60, duration: 1200 });
  }, [map, trajectoryPoints]);

  return null;
}

export function TrajectoryMap({ predictionData }: TrajectoryMapProps) {
  const allPoints = useMemo(() => {
    if (!predictionData) return null;
    const ascent: TrajectoryPoint[] = predictionData.ascent_path ?? [];
    const descent: TrajectoryPoint[] = predictionData.descent_path ?? [];
    return [...ascent, ...descent.slice(1)];
  }, [predictionData]);

  const deckLayers = useMemo(() => {
    if (!predictionData) return [];

    const ascentPath: [number, number, number][] = (predictionData.ascent_path ?? [])
      .map((p: TrajectoryPoint) => [p.lon, p.lat, p.alt]);
    const descentPath: [number, number, number][] = (predictionData.descent_path ?? [])
      .map((p: TrajectoryPoint) => [p.lon, p.lat, p.alt]);

    const layers: any[] = [];

    if (ascentPath.length >= 2) {
      layers.push(
        new PathLayer({
          id: 'ascent-line-3d',
          data: [{ path: ascentPath }],
          getPath: (d: { path: [number, number, number][] }) => d.path,
          getColor: [239, 68, 68, 220],

          widthUnits: 'pixels',
          getWidth: 6,
          
          billboard: true,
          rounded: true,
          jointRounded: true,
        })
      );
    }

    if (descentPath.length >= 2) {
      layers.push(
        new PathLayer({
          id: 'descent-line-3d',
          data: [{ path: descentPath }],
          getPath: (d: { path: [number, number, number][] }) => d.path,
          getColor: [59, 130, 246, 220],

          widthUnits: 'pixels',
          getWidth: 6,

          billboard: true,
          rounded: true,
          jointRounded: true,
        })
      );
    }

    return layers;
  }, [predictionData]);

  const launchPos = useMemo(() => {
    if (!predictionData?.ascent_path?.length) return null;
    const p = predictionData.ascent_path[0];
    return { lat: p.lat, lon: p.lon };
  }, [predictionData]);

  const landingPos = useMemo(() => {
    if (!predictionData) return null;
    return { lat: predictionData.landing_lat, lon: predictionData.landing_lon };
  }, [predictionData]);

  return (
    <div className="w-full h-full">
      <Map
        initialViewState={{
          longitude: 135,
          latitude: 35,
          zoom: 5,
        }}
        style={{ width: "100%", height: "100%" }}
        mapStyle="https://tiles.openfreemap.org/styles/liberty"
      >
        <MapController trajectoryPoints={allPoints} />

        {deckLayers.length > 0 && (
          <DeckGLOverlay layers={deckLayers} interleaved={true} />
        )}

        {launchPos && (
          <Marker longitude={launchPos.lon} latitude={launchPos.lat} anchor="bottom">
            <div className="relative">
              <div className="w-3 h-3 bg-red-500 border-2 border-white rounded-full shadow-lg" />
              <span className="absolute -top-5 left-1/2 -translate-x-1/2 text-[10px] font-bold bg-background/80 px-1 rounded whitespace-nowrap">
                放球
              </span>
            </div>
          </Marker>
        )}

        {landingPos && (
          <Marker longitude={landingPos.lon} latitude={landingPos.lat} anchor="bottom">
            <div className="relative">
              <div className="w-3 h-3 bg-blue-500 border-2 border-white rounded-full shadow-lg" />
              <span className="absolute -top-5 left-1/2 -translate-x-1/2 text-[10px] font-bold bg-background/80 px-1 rounded whitespace-nowrap">
                着地
              </span>
            </div>
          </Marker>
        )}
      </Map>
    </div>
  );
}