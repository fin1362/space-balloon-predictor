import { SidebarProvider, SidebarTrigger } from "@/components/ui/sidebar"
import { AppSidebar } from "@/components/app-sidebar"
import { Map } from '@vis.gl/react-maplibre';
import 'maplibre-gl/dist/maplibre-gl.css';


function App() {
  return (
    <SidebarProvider>
      <AppSidebar />
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
          mapStyle="https://demotiles.maplibre.org/style.json"
        />
      </main>
    </SidebarProvider>
  )
}

export default App
