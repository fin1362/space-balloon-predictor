import { SidebarProvider, SidebarTrigger } from "@/components/ui/sidebar"
import { AppSidebar, PredictorFormValues } from "@/components/app-sidebar"
import { Map } from '@vis.gl/react-maplibre';
import 'maplibre-gl/dist/maplibre-gl.css';
import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";

function App() {
  const [_predictionData, setPredictionData] = useState<any>(null)
  const [loading, setLoading] = useState(false)

  const handlePredict = async (values: PredictorFormValues) => {
    setLoading(true)
    try {
      const [latStr, lonStr] = values.launchPosition.split(",").map(s => s.trim());
      const launchLat = parseFloat(latStr);
      const launchLon = parseFloat(lonStr);

      if (isNaN(launchLat) || isNaN(launchLon)) {
        throw new Error("放球位置は '緯度,経度' の形式で入力してください");
      }

      const launchDate = values.launchDate!;
      const [hours, minutes] = values.launchTime.split(":").map(Number);
      const launchDateTime = new Date(launchDate);
      launchDateTime.setUTCHours(hours, minutes, 0, 0);

      const gfsRunTime = new Date(launchDateTime.getTime() - 12 * 60 * 60 * 1000);

      const result = await invoke("run_simulation", {
        launchLat,
        launchLon,
        launchAlt: 10.0,
        gfsRunTime: gfsRunTime.toISOString(),
        launchTime: launchDateTime.toISOString(),
        ascentRate: Number(values.ascentRate),
        descentRate: Number(values.descentRate),
        burstAltitude: Number(values.burstAltitude),
      });

      console.log("Simulation result:", result);
      setPredictionData(result);
    } catch (e) {
      console.error("Simulation failed:", e);
    } finally {
      setLoading(false);
    }
  }

  return (
    <SidebarProvider>
      <AppSidebar onSubmit={handlePredict} isLoading={loading} />
      <main className="absolute inset-0 w-screen h-screen z-0">
        <div className="absolute top-4 left-4 z-20">
          <SidebarTrigger className="bg-background border shadow-sm" />
        </div>

        <Map
          initialViewState={{
            longitude: -100,
            latitude: 40,
            zoom: 3.5
          }}
          // 親の main が絶対配置で画面一杯になっているため、100% 指定で綺麗に収まります
          style={{ width: "100%", height: "100%" }}
          mapStyle="https://tiles.openfreemap.org/styles/liberty"
        />
      </main>
    </SidebarProvider>
  )
}

export default App
