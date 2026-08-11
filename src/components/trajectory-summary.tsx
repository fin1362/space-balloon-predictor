import { useMemo } from "react"
import { Navigation, Timer, Gauge } from "lucide-react"
import { AltitudeTimeChart } from "@/components/altitude-time-chart"
import { findClosestSigmaIndex, haversineKm } from "@/lib/geo"
import type { MonteCarloResult, PredictionData } from "@/types"

interface TrajectorySummaryProps {
  predictionData: PredictionData | null
  monteCarloData?: MonteCarloResult | null
  selectedPointIndex?: number | null
  launchLat?: number
  launchLon?: number
}

function formatDuration(totalS: number | undefined): string {
  if (totalS === undefined || isNaN(totalS) || totalS <= 0) return "—"
  const totalMin = Math.round(totalS / 60)
  const h = Math.floor(totalMin / 60)
  const m = totalMin % 60
  return `${h}時間${m}分`
}

function sigmaColor(sigma: number): string {
  const abs = Math.abs(sigma)
  if (abs <= 1) return "bg-[rgb(34,197,94)]"
  if (abs <= 2) return "bg-[rgb(234,179,8)]"
  return "bg-[rgb(239,68,68)]"
}

function StatRow({
  icon,
  label,
  value,
  unit,
  dotColor,
}: {
  icon: React.ReactNode
  label: string
  value: string
  unit?: string
  dotColor?: string
}) {
  return (
    <div className="flex items-center justify-between gap-2">
      <span className="flex items-center gap-1.5 text-muted-foreground">
        {icon}
        <span className="text-xs">{label}</span>
      </span>
      <span className="flex items-center gap-1.5 font-medium tabular-nums">
        {dotColor && <span className={`w-2 h-2 rounded-full ${dotColor}`} />}
        {value}
        {unit && <span className="text-xs text-muted-foreground font-normal">{unit}</span>}
      </span>
    </div>
  )
}

export function TrajectorySummary({
  predictionData,
  monteCarloData = null,
  selectedPointIndex = null,
  launchLat,
  launchLon,
}: TrajectorySummaryProps) {
  const activeSelectedIndex = useMemo(() => {
    if (!monteCarloData) return null
    if (selectedPointIndex !== null && selectedPointIndex !== undefined) {
      return selectedPointIndex
    }
    return findClosestSigmaIndex(monteCarloData.points)
  }, [selectedPointIndex, monteCarloData])

  const isMonteCarlo = !!monteCarloData

  const point = useMemo(() => {
    if (!monteCarloData) return null
    if (activeSelectedIndex === null || activeSelectedIndex === undefined) return null
    return monteCarloData.points[activeSelectedIndex] ?? null
  }, [monteCarloData, activeSelectedIndex])

  if (!isMonteCarlo && !predictionData) {
    return
  }

  if (isMonteCarlo && !point) {
    return 
  }

  const single = predictionData

  const drift =
    isMonteCarlo && point && launchLat != null && launchLon != null
      ? haversineKm(launchLat, launchLon, point.landing_lat, point.landing_lon)
      : single?.drift_km

  const burstAlt = isMonteCarlo ? point?.burst_altitude : single?.max_altitude

  const ascent =
    isMonteCarlo && point
      ? monteCarloData.trajectories?.[activeSelectedIndex!]?.ascent_path ?? []
      : single?.ascent_path ?? []
  const descent =
    isMonteCarlo && point
      ? monteCarloData.trajectories?.[activeSelectedIndex!]?.descent_path ?? []
      : single?.descent_path ?? []

  const totalS = isMonteCarlo
    ? descent[descent.length - 1]?.time ?? ascent[ascent.length - 1]?.time
    : single?.total_duration_s

  const meanAscent = isMonteCarlo ? monteCarloData.mean_ascent_path : undefined
  const meanDescent = isMonteCarlo ? monteCarloData.mean_descent_path : undefined

  return (
    <div className="absolute right-4 bottom-12 z-20 bg-background border rounded-lg p-3 text-sm space-y-1.5 w-72 shadow-sm">
      <p className="font-medium text-sm mb-1">
        軌道の概要
      </p>

      <StatRow
        icon={<Gauge className="size-3.5" />}
        label="バースト高度"
        value={burstAlt !== undefined ? Math.round(burstAlt).toLocaleString() : "—"}
        unit="m"
      />
      <StatRow
        icon={<Timer className="size-3.5" />}
        label="飛行時間"
        value={formatDuration(totalS)}
      />
      <StatRow
        icon={<Navigation className="size-3.5" />}
        label="水平距離"
        value={drift !== undefined ? drift.toFixed(1) : "—"}
        unit="km"
      />
      {isMonteCarlo && point && (
        <StatRow
          icon={<SigmaIcon />}
          label="偏差 σ"
          value={`${point.deviation_sigma >= 0 ? "+" : ""}${point.deviation_sigma.toFixed(2)}`}
          dotColor={sigmaColor(point.deviation_sigma)}
        />
      )}

      <div className="border-t pt-2">
        <AltitudeTimeChart
          ascent={ascent}
          descent={descent}
          meanAscent={meanAscent}
          meanDescent={meanDescent}
        />
      </div>
    </div>
  )
}

function SigmaIcon() {
  return (
    <svg
      className="size-3.5"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      <path d="M18 7V5a1 1 0 0 0-1-1H6.5a.5.5 0 0 0-.4.8l.9 1.2a.5.5 0 0 1 0 .6L6 8a.5.5 0 0 0 0 .6L5 9.8a.5.5 0 0 0 0 .6l.2" />
    </svg>
  )
}
