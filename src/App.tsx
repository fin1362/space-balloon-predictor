import { SidebarProvider, SidebarTrigger } from "@/components/ui/sidebar"
import { AppSidebar, PredictorFormValues, PositionMode, PRESETS } from "@/components/app-sidebar"
import { TrajectoryMap } from "@/components/trajectory-map"
import { useState } from "react"
import { invoke } from "@tauri-apps/api/core"

const DEFAULT_LAT = PRESETS[0].lat
const DEFAULT_LON = PRESETS[0].lon

function App() {
  const [predictionData, setPredictionData] = useState<any>(null)
  const [loading, setLoading] = useState(false)
  const [positionMode, setPositionMode] = useState<PositionMode>("preset")
  const [launchLat, setLaunchLat] = useState<number>(DEFAULT_LAT)
  const [launchLon, setLaunchLon] = useState<number>(DEFAULT_LON)

  const handlePredict = async (values: PredictorFormValues) => {
    setLoading(true)
    try {
      const launchDate = values.launchDate!
      const [hours, minutes] = values.launchTime.split(":").map(Number)
      const launchDateTime = new Date(launchDate)
      launchDateTime.setUTCHours(hours, minutes, 0, 0)

      const gfsRunTime = new Date(
        launchDateTime.getTime() - 12 * 60 * 60 * 1000,
      )

      const result = await invoke("run_simulation", {
        launchLat: values.launchLat,
        launchLon: values.launchLon,
        launchAlt: 10.0,
        gfsRunTime: gfsRunTime.toISOString(),
        launchTime: launchDateTime.toISOString(),
        ascentRate: Number(values.ascentRate),
        descentRate: Number(values.descentRate),
        burstAltitude: Number(values.burstAltitude),
      })

      console.log("Simulation result:", result)
      setPredictionData(result)
    } catch (e) {
      console.error("Simulation failed:", e)
    } finally {
      setLoading(false)
    }
  }

  const handleLaunchPositionChange = (lat: number, lon: number) => {
    setLaunchLat(lat)
    setLaunchLon(lon)
  }

  return (
    <SidebarProvider>
      <AppSidebar
        onSubmit={handlePredict}
        isLoading={loading}
        launchLat={launchLat}
        launchLon={launchLon}
        onLaunchPositionChange={handleLaunchPositionChange}
        positionMode={positionMode}
        onPositionModeChange={setPositionMode}
      />
      <main className="absolute inset-0 w-screen h-screen z-0">
        <div className="absolute top-4 left-4 z-20">
          <SidebarTrigger className="bg-background border shadow-sm" />
        </div>
        <TrajectoryMap
          predictionData={predictionData}
          launchLat={launchLat}
          launchLon={launchLon}
          mapSelectionMode={positionMode === "map"}
          onMapClick={(lat, lon) => {
            setLaunchLat(lat)
            setLaunchLon(lon)
            setPositionMode("preset") // 地図クリック後は自動的にプリセット（または選択済み）モードへ切り替え
          }}
        />
      </main>
    </SidebarProvider>
  )
}

export default App