import React, { useRef } from 'react';
import { StyleSheet, Text, View } from 'react-native';
import Svg from 'react-native-svg';
import { Gesture, GestureDetector } from 'react-native-gesture-handler';
import { Ring } from './Ring';
import { angleToFraction, fractionToValue, pointToAngle, type RingFractions } from '../lib/time';

export type RingUnit = 'hour' | 'minute' | 'second';

interface Geometry {
  size: number;
  centerXY: number;
  strokeWidth: number;
  radii: Record<RingUnit, number>;
  bandBoundaries: { center: number; secondMid: number; minuteMid: number };
}

function computeGeometry(size: number): Geometry {
  const strokeWidth = size * 0.045;
  const gap = size * 0.02;
  const outerMargin = size * 0.025;

  const hour = size / 2 - outerMargin - strokeWidth / 2;
  const minute = hour - strokeWidth - gap;
  const second = minute - strokeWidth - gap;
  const center = second - strokeWidth / 2 - gap * 1.5;

  return {
    size,
    centerXY: size / 2,
    strokeWidth,
    radii: { hour, minute, second },
    bandBoundaries: {
      center,
      secondMid: (second + minute) / 2,
      minuteMid: (minute + hour) / 2,
    },
  };
}

function classifyZone(geometry: Geometry, dx: number, dy: number): RingUnit | 'center' | null {
  const distance = Math.sqrt(dx * dx + dy * dy);
  const { center, secondMid, minuteMid } = geometry.bandBoundaries;
  if (distance <= center) return 'center';
  if (distance <= secondMid) return 'second';
  if (distance <= minuteMid) return 'minute';
  if (distance <= geometry.size / 2 + geometry.strokeWidth) return 'hour';
  return null;
}

const RING_COLORS: Record<RingUnit, string> = {
  hour: '#9B8CE8',
  minute: '#E8AA4C',
  second: '#4FD1C5',
};
const TRACK_COLOR = 'rgba(255,255,255,0.08)';

const LONG_PRESS_MS = 550;
const MOVE_TOLERANCE = 14;

export interface ThreeRingDialProps {
  size: number;
  fractions: RingFractions;
  centerLabel: string;
  caption?: string;
  interactive: boolean;
  onCenterTap: () => void;
  onCenterLongPress: () => void;
  onRingDrag?: (unit: RingUnit, value: number) => void;
}

export function ThreeRingDial({
  size,
  fractions,
  centerLabel,
  caption,
  interactive,
  onCenterTap,
  onCenterLongPress,
  onRingDrag,
}: ThreeRingDialProps) {
  const geometry = computeGeometry(size);
  const gestureState = useRef<{
    zone: RingUnit | 'center' | null;
    startTime: number;
    maxMovement: number;
  }>({ zone: null, startTime: 0, maxMovement: 0 });

  const handleBegin = (x: number, y: number) => {
    const dx = x - geometry.centerXY;
    const dy = y - geometry.centerXY;
    gestureState.current = {
      zone: classifyZone(geometry, dx, dy),
      startTime: Date.now(),
      maxMovement: 0,
    };
  };

  const handleUpdate = (x: number, y: number) => {
    const dx = x - geometry.centerXY;
    const dy = y - geometry.centerXY;
    const state = gestureState.current;
    state.maxMovement = Math.max(state.maxMovement, Math.sqrt(dx * dx + dy * dy) - geometry.bandBoundaries.center);

    if (!interactive || !onRingDrag) return;
    if (state.zone === 'center' || state.zone === null) return;
    const angle = pointToAngle(dx, dy);
    const fraction = angleToFraction(angle);
    const max = state.zone === 'hour' ? 24 : 60;
    onRingDrag(state.zone, fractionToValue(fraction, max));
  };

  const handleEnd = () => {
    const state = gestureState.current;
    if (state.zone !== 'center') return;
    const elapsed = Date.now() - state.startTime;
    if (state.maxMovement > MOVE_TOLERANCE) return;
    if (elapsed >= LONG_PRESS_MS) {
      onCenterLongPress();
    } else {
      onCenterTap();
    }
  };

  const pan = Gesture.Pan()
    .runOnJS(true)
    .minDistance(0)
    .onBegin((e) => handleBegin(e.x, e.y))
    .onUpdate((e) => handleUpdate(e.x, e.y))
    .onEnd(() => handleEnd());

  return (
    <GestureDetector gesture={pan}>
      <View style={{ width: size, height: size }}>
        <Svg width={size} height={size}>
          {(['hour', 'minute', 'second'] as RingUnit[]).map((unit) => (
            <Ring
              key={unit}
              cx={geometry.centerXY}
              cy={geometry.centerXY}
              radius={geometry.radii[unit]}
              strokeWidth={geometry.strokeWidth}
              fraction={fractions[unit]}
              color={RING_COLORS[unit]}
              trackColor={TRACK_COLOR}
            />
          ))}
        </Svg>
        <View style={StyleSheet.absoluteFill} pointerEvents="none">
          <View style={styles.centerContent}>
            <Text style={styles.centerLabel} numberOfLines={1} adjustsFontSizeToFit>
              {centerLabel}
            </Text>
            {caption ? <Text style={styles.caption}>{caption}</Text> : null}
          </View>
        </View>
      </View>
    </GestureDetector>
  );
}

const styles = StyleSheet.create({
  centerContent: {
    flex: 1,
    alignItems: 'center',
    justifyContent: 'center',
  },
  centerLabel: {
    color: '#F5F5F7',
    fontSize: 52,
    fontWeight: '300',
    fontVariant: ['tabular-nums'],
    letterSpacing: 1,
  },
  caption: {
    marginTop: 8,
    color: 'rgba(245,245,247,0.4)',
    fontSize: 12,
    letterSpacing: 1.5,
    textTransform: 'uppercase',
  },
});
