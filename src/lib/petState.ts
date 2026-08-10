export type PetMood = "sleepy" | "working" | "ready" | "paid" | "wild";

export function moodFromPaybackRatio(ratio: number): PetMood {
  if (ratio >= 2) return "wild";
  if (ratio >= 1) return "paid";
  if (ratio >= 0.8) return "ready";
  if (ratio >= 0.3) return "working";
  return "sleepy";
}
