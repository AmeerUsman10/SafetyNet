# SafetyNet

A minimal, aesthetic countdown timer built with Expo/React Native.

Three concentric rings — hour, minute, second — double as an analog clock face.
Tap the center to turn it into a countdown: drag around each ring to set hours,
minutes, and seconds, then tap again to start. The screen stays awake while a
countdown runs, so it keeps showing on a locked/dimmed phone. Three distinct
vibration patterns mark each second, each minute, and completion, with an
optional tick-tock sound.

## Interaction

- **Tap center** — clock → set duration → start countdown → pause → resume →
  (on completion) tap to return to the clock.
- **Drag a ring** (while setting the duration) — sets that ring's unit; angle
  around the ring maps to the value (hour: 0–23, minute/second: 0–59).
- **Long-press center** — reset to the clock from any countdown state.
- **⚙ (top-right)** — toggle tick-tock sound. Setting is persisted.

## Getting started

```sh
npm install
npm run web    # or: npm run ios / npm run android
```

Sound assets are synthesized (not recorded) so there are no external audio
files to license. Regenerate them with:

```sh
node scripts/gen-sounds.mjs
```

## Project layout

- `src/screens/TimerScreen.tsx` — state machine wiring the dial, haptics,
  sound, keep-awake, and settings together.
- `src/components/ThreeRingDial.tsx` / `Ring.tsx` — the SVG dial and its
  tap/drag gesture handling.
- `src/hooks/useCountdownEngine.ts` — drift-free countdown timer.
- `src/lib/` — time/angle math, haptics, sound playback, settings storage.
