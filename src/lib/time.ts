export const SECONDS_MAX = 60;
export const MINUTES_MAX = 60;
export const HOURS_MAX = 24;

export interface RingFractions {
  hour: number;
  minute: number;
  second: number;
}

/** Analog "hand" fractions (0..1) for a given total-seconds value, used identically
 * by the live clock and the countdown so both modes share one visual language. */
export function ringFractionsFromSeconds(totalSeconds: number): RingFractions {
  const s = Math.max(0, Math.floor(totalSeconds));
  return {
    second: (s % SECONDS_MAX) / SECONDS_MAX,
    minute: (Math.floor(s / SECONDS_MAX) % MINUTES_MAX) / MINUTES_MAX,
    hour: (Math.floor(s / (SECONDS_MAX * MINUTES_MAX)) % HOURS_MAX) / HOURS_MAX,
  };
}

export function clockFractionsFromDate(date: Date): RingFractions {
  const seconds = date.getSeconds() + date.getMilliseconds() / 1000;
  return {
    second: seconds / SECONDS_MAX,
    minute: (date.getMinutes() + seconds / SECONDS_MAX) / MINUTES_MAX,
    hour: ((date.getHours() % HOURS_MAX) + date.getMinutes() / MINUTES_MAX) / HOURS_MAX,
  };
}

export function pad2(n: number): string {
  return n.toString().padStart(2, '0');
}

export function formatClock(date: Date): string {
  return `${pad2(date.getHours())}:${pad2(date.getMinutes())}:${pad2(date.getSeconds())}`;
}

export interface DurationParts {
  hours: number;
  minutes: number;
  seconds: number;
}

export function durationToSeconds({ hours, minutes, seconds }: DurationParts): number {
  return hours * 3600 + minutes * 60 + seconds;
}

export function secondsToDurationParts(totalSeconds: number): DurationParts {
  const s = Math.max(0, Math.floor(totalSeconds));
  return {
    hours: Math.floor(s / 3600) % HOURS_MAX,
    minutes: Math.floor(s / 60) % MINUTES_MAX,
    seconds: s % SECONDS_MAX,
  };
}

/** Format remaining/entered duration, shrinking to just the units in play so the
 * center readout stays uncluttered (e.g. "45" instead of "00:00:45"). */
export function formatDuration(totalSeconds: number): string {
  const { hours, minutes, seconds } = secondsToDurationParts(totalSeconds);
  if (hours > 0) return `${hours}:${pad2(minutes)}:${pad2(seconds)}`;
  if (minutes > 0) return `${minutes}:${pad2(seconds)}`;
  return `${seconds}`;
}

/** Converts an angle (radians, 0 = up/12 o'clock, clockwise-positive) to a fraction 0..1. */
export function angleToFraction(angleRad: number): number {
  const normalized = (angleRad + Math.PI * 2) % (Math.PI * 2);
  return normalized / (Math.PI * 2);
}

/** Angle from a dial's center to a touch point, matching angleToFraction's convention
 * (0 at the top, increasing clockwise). Standard screen coordinates: y grows downward. */
export function pointToAngle(dx: number, dy: number): number {
  return Math.atan2(dx, -dy);
}

export function fractionToValue(fraction: number, max: number): number {
  const v = Math.round(fraction * max) % max;
  return v < 0 ? v + max : v;
}
