import { MonteCarloPoint } from "@/types"

export const EARTH_RADIUS_KM = 6371.0

export function haversineKm(
  lat1: number,
  lon1: number,
  lat2: number,
  lon2: number,
): number {
  const dLat = ((lat2 - lat1) * Math.PI) / 180
  const dLon = ((lon2 - lon1) * Math.PI) / 180
  const a =
    Math.sin(dLat / 2) ** 2 +
    Math.cos((lat1 * Math.PI) / 180) *
      Math.cos((lat2 * Math.PI) / 180) *
      Math.sin(dLon / 2) ** 2
  const c = 2 * Math.atan2(Math.sqrt(a), Math.sqrt(1 - a))
  return EARTH_RADIUS_KM * c
}

export function findClosestSigmaIndex(points: MonteCarloPoint[]): number | null {
  if (!points || points.length === 0) return null
  let bestIdx = 0
  let minDiff = Math.abs(points[0]?.deviation_sigma ?? 0)
  for (let i = 1; i < points.length; i++) {
    const diff = Math.abs(points[i]?.deviation_sigma ?? 0)
    if (diff < minDiff) {
      minDiff = diff
      bestIdx = i
    }
  }
  return bestIdx
}