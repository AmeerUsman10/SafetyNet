import AsyncStorage from '@react-native-async-storage/async-storage';

const SOUND_ENABLED_KEY = 'safetynet.soundEnabled';

export async function loadSoundEnabled(): Promise<boolean> {
  try {
    const raw = await AsyncStorage.getItem(SOUND_ENABLED_KEY);
    return raw === 'true';
  } catch {
    return false;
  }
}

export async function saveSoundEnabled(value: boolean): Promise<void> {
  try {
    await AsyncStorage.setItem(SOUND_ENABLED_KEY, value ? 'true' : 'false');
  } catch {
    // Non-critical: worst case the toggle doesn't persist across launches.
  }
}
