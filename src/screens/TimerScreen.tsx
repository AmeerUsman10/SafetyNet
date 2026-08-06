import React, { useCallback, useEffect, useMemo, useState } from 'react';
import { Pressable, StyleSheet, Text, useWindowDimensions, View } from 'react-native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import { activateKeepAwakeAsync, deactivateKeepAwake } from 'expo-keep-awake';
import { ThreeRingDial, type RingUnit } from '../components/ThreeRingDial';
import { SettingsSheet } from '../components/SettingsSheet';
import { useClock } from '../hooks/useClock';
import { useCountdownEngine } from '../hooks/useCountdownEngine';
import { completeHaptic, minuteHaptic, tickHaptic } from '../lib/haptics';
import { loadSoundEnabled, saveSoundEnabled } from '../lib/settingsStore';
import { playComplete, playTick, setSoundEnabled } from '../lib/sound';
import {
  clockFractionsFromDate,
  durationToSeconds,
  formatClock,
  formatDuration,
  ringFractionsFromSeconds,
  type DurationParts,
} from '../lib/time';

type Phase = 'clock' | 'setting' | 'running' | 'paused' | 'finished';

const KEEP_AWAKE_TAG = 'safetynet-countdown';
const EMPTY_DURATION: DurationParts = { hours: 0, minutes: 0, seconds: 0 };

export function TimerScreen() {
  const { width, height } = useWindowDimensions();
  const insets = useSafeAreaInsets();

  const [phase, setPhase] = useState<Phase>('clock');
  const [duration, setDuration] = useState<DurationParts>(EMPTY_DURATION);
  const [soundEnabledState, setSoundEnabledState] = useState(false);
  const [settingsVisible, setSettingsVisible] = useState(false);

  const now = useClock();

  const countdown = useCountdownEngine({
    onTick: () => {
      tickHaptic();
      playTick();
    },
    onMinuteBoundary: () => {
      minuteHaptic();
    },
    onComplete: () => {
      completeHaptic();
      playComplete();
      setPhase('finished');
    },
  });

  useEffect(() => {
    loadSoundEnabled().then((enabled) => {
      setSoundEnabledState(enabled);
      setSoundEnabled(enabled);
    });
  }, []);

  useEffect(() => {
    const shouldStayAwake = phase === 'running' || phase === 'paused' || phase === 'finished';
    if (shouldStayAwake) {
      activateKeepAwakeAsync(KEEP_AWAKE_TAG).catch(() => {});
    } else {
      deactivateKeepAwake(KEEP_AWAKE_TAG).catch(() => {});
    }
    return () => {
      deactivateKeepAwake(KEEP_AWAKE_TAG).catch(() => {});
    };
  }, [phase]);

  const handleToggleSound = useCallback((value: boolean) => {
    setSoundEnabledState(value);
    setSoundEnabled(value);
    saveSoundEnabled(value);
  }, []);

  const handleCenterTap = useCallback(() => {
    if (phase === 'clock') {
      setDuration(EMPTY_DURATION);
      setPhase('setting');
      return;
    }
    if (phase === 'setting') {
      const totalSeconds = durationToSeconds(duration);
      if (totalSeconds <= 0) return;
      countdown.start(totalSeconds);
      setPhase('running');
      return;
    }
    if (phase === 'running') {
      countdown.pause();
      setPhase('paused');
      return;
    }
    if (phase === 'paused') {
      countdown.resume();
      setPhase('running');
      return;
    }
    if (phase === 'finished') {
      countdown.reset();
      setPhase('clock');
    }
  }, [phase, duration, countdown]);

  const handleCenterLongPress = useCallback(() => {
    if (phase === 'clock') return;
    countdown.reset();
    setPhase('clock');
  }, [phase, countdown]);

  const handleRingDrag = useCallback((unit: RingUnit, value: number) => {
    setDuration((prev) => ({
      ...prev,
      [unit === 'hour' ? 'hours' : unit === 'minute' ? 'minutes' : 'seconds']: value,
    }));
  }, []);

  const fractions = useMemo(() => {
    if (phase === 'clock') return clockFractionsFromDate(now);
    if (phase === 'setting') return ringFractionsFromSeconds(durationToSeconds(duration));
    return ringFractionsFromSeconds(countdown.remainingSeconds);
  }, [phase, now, duration, countdown.remainingSeconds]);

  const centerLabel = useMemo(() => {
    if (phase === 'clock') return formatClock(now);
    if (phase === 'setting') return formatDuration(durationToSeconds(duration));
    if (phase === 'finished') return 'Done';
    return formatDuration(countdown.remainingSeconds);
  }, [phase, now, duration, countdown.remainingSeconds]);

  const caption = useMemo(() => {
    if (phase === 'setting') return 'drag to set · tap to start';
    if (phase === 'paused') return 'paused';
    return undefined;
  }, [phase]);

  const dialSize = Math.min(width, height) * 0.82;
  const showSettingsGear = phase === 'clock' || phase === 'setting' || phase === 'paused';

  return (
    <View style={styles.container}>
      {showSettingsGear && (
        <Pressable
          style={[styles.settingsButton, { top: insets.top + 12 }]}
          onPress={() => setSettingsVisible(true)}
          hitSlop={12}
        >
          <Text style={styles.settingsGlyph}>⚙</Text>
        </Pressable>
      )}

      <View style={styles.center}>
        <ThreeRingDial
          size={dialSize}
          fractions={fractions}
          centerLabel={centerLabel}
          caption={caption}
          interactive={phase === 'setting'}
          onCenterTap={handleCenterTap}
          onCenterLongPress={handleCenterLongPress}
          onRingDrag={handleRingDrag}
        />
      </View>

      <SettingsSheet
        visible={settingsVisible}
        soundEnabled={soundEnabledState}
        onToggleSound={handleToggleSound}
        onClose={() => setSettingsVisible(false)}
      />
    </View>
  );
}

const styles = StyleSheet.create({
  container: {
    flex: 1,
    backgroundColor: '#0B0B0F',
  },
  center: {
    flex: 1,
    alignItems: 'center',
    justifyContent: 'center',
  },
  settingsButton: {
    position: 'absolute',
    right: 16,
    zIndex: 10,
    padding: 8,
    opacity: 0.35,
  },
  settingsGlyph: {
    color: '#F5F5F7',
    fontSize: 20,
  },
});
