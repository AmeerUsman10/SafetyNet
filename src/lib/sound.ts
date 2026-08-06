import { createAudioPlayer, type AudioPlayer } from 'expo-audio';

const tickSource = require('../../assets/sounds/tick.wav');
const tockSource = require('../../assets/sounds/tock.wav');
const completeSource = require('../../assets/sounds/complete.wav');

let tickPlayer: AudioPlayer | null = null;
let tockPlayer: AudioPlayer | null = null;
let completePlayer: AudioPlayer | null = null;
let enabled = false;

function ensurePlayers() {
  if (!tickPlayer) tickPlayer = createAudioPlayer(tickSource);
  if (!tockPlayer) tockPlayer = createAudioPlayer(tockSource);
  if (!completePlayer) completePlayer = createAudioPlayer(completeSource);
}

function replay(player: AudioPlayer | null) {
  if (!player) return;
  player.seekTo(0);
  player.play();
}

export function setSoundEnabled(next: boolean) {
  enabled = next;
  if (enabled) ensurePlayers();
}

/** Alternates tick/tock each call so a running countdown reads as a clock, not a metronome. */
let alternate = false;
export function playTick() {
  if (!enabled) return;
  ensurePlayers();
  alternate = !alternate;
  replay(alternate ? tickPlayer : tockPlayer);
}

export function playComplete() {
  if (!enabled) return;
  ensurePlayers();
  replay(completePlayer);
}
