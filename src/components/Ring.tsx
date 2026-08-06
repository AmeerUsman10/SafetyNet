import React from 'react';
import { Circle } from 'react-native-svg';

interface RingProps {
  cx: number;
  cy: number;
  radius: number;
  strokeWidth: number;
  fraction: number; // 0..1, clockwise fill starting at 12 o'clock
  color: string;
  trackColor: string;
}

export function Ring({ cx, cy, radius, strokeWidth, fraction, color, trackColor }: RingProps) {
  const circumference = 2 * Math.PI * radius;
  const clamped = Math.min(1, Math.max(0, fraction));
  const dashOffset = circumference * (1 - clamped);

  return (
    <>
      <Circle cx={cx} cy={cy} r={radius} stroke={trackColor} strokeWidth={strokeWidth} fill="none" />
      {clamped > 0 && (
        <Circle
          cx={cx}
          cy={cy}
          r={radius}
          stroke={color}
          strokeWidth={strokeWidth}
          fill="none"
          strokeLinecap="round"
          strokeDasharray={`${circumference} ${circumference}`}
          strokeDashoffset={dashOffset}
          transform={`rotate(-90 ${cx} ${cy})`}
        />
      )}
    </>
  );
}
