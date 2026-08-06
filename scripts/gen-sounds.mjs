// One-off generator for the app's tick/tock/complete sound effects.
// Synthesizes short PCM16 WAV tones so no external/copyrighted audio assets are needed.
// Run with: node scripts/gen-sounds.mjs
import { writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

const SAMPLE_RATE = 44100;
const OUT_DIR = path.join(path.dirname(fileURLToPath(import.meta.url)), '..', 'assets', 'sounds');

function encodeWav(samples) {
  const bytesPerSample = 2;
  const blockAlign = bytesPerSample;
  const dataSize = samples.length * bytesPerSample;
  const buffer = Buffer.alloc(44 + dataSize);

  buffer.write('RIFF', 0);
  buffer.writeUInt32LE(36 + dataSize, 4);
  buffer.write('WAVE', 8);
  buffer.write('fmt ', 12);
  buffer.writeUInt32LE(16, 16); // fmt chunk size
  buffer.writeUInt16LE(1, 20); // PCM
  buffer.writeUInt16LE(1, 22); // mono
  buffer.writeUInt32LE(SAMPLE_RATE, 24);
  buffer.writeUInt32LE(SAMPLE_RATE * blockAlign, 28); // byte rate
  buffer.writeUInt16LE(blockAlign, 32);
  buffer.writeUInt16LE(16, 34); // bits per sample
  buffer.write('data', 36);
  buffer.writeUInt32LE(dataSize, 40);

  for (let i = 0; i < samples.length; i++) {
    const clamped = Math.max(-1, Math.min(1, samples[i]));
    buffer.writeInt16LE(Math.round(clamped * 32767), 44 + i * bytesPerSample);
  }
  return buffer;
}

// Fast attack, exponential decay envelope — reads as a crisp "click" rather than a pure tone.
function envelope(t, durationSec, attackSec = 0.002, decayRate = 18) {
  if (t < attackSec) return t / attackSec;
  return Math.exp(-decayRate * (t - attackSec));
}

function tone(freqHz, durationSec, { decayRate = 18, gain = 0.9 } = {}) {
  const n = Math.round(SAMPLE_RATE * durationSec);
  const samples = new Float32Array(n);
  for (let i = 0; i < n; i++) {
    const t = i / SAMPLE_RATE;
    samples[i] = Math.sin(2 * Math.PI * freqHz * t) * envelope(t, durationSec, 0.002, decayRate) * gain;
  }
  return samples;
}

function concat(...parts) {
  const total = parts.reduce((sum, p) => sum + p.length, 0);
  const out = new Float32Array(total);
  let offset = 0;
  for (const p of parts) {
    out.set(p, offset);
    offset += p.length;
  }
  return out;
}

function silence(durationSec) {
  return new Float32Array(Math.round(SAMPLE_RATE * durationSec));
}

// Bright short click for the "tick" (odd seconds feel).
const tick = tone(1800, 0.05, { decayRate: 40, gain: 0.85 });
// Slightly lower, marginally longer click for "tock" (even seconds feel).
const tock = tone(1150, 0.06, { decayRate: 34, gain: 0.85 });
// Gentle three-note ascending chime for countdown completion.
const complete = concat(
  tone(659.25, 0.16, { decayRate: 10, gain: 0.8 }), // E5
  silence(0.02),
  tone(783.99, 0.16, { decayRate: 10, gain: 0.8 }), // G5
  silence(0.02),
  tone(1046.5, 0.28, { decayRate: 7, gain: 0.85 }) // C6
);

writeFileSync(path.join(OUT_DIR, 'tick.wav'), encodeWav(tick));
writeFileSync(path.join(OUT_DIR, 'tock.wav'), encodeWav(tock));
writeFileSync(path.join(OUT_DIR, 'complete.wav'), encodeWav(complete));

console.log('Wrote tick.wav, tock.wav, complete.wav to', OUT_DIR);
