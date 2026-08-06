import React from 'react';
import { Modal, Pressable, StyleSheet, Switch, Text, View } from 'react-native';

interface SettingsSheetProps {
  visible: boolean;
  soundEnabled: boolean;
  onToggleSound: (value: boolean) => void;
  onClose: () => void;
}

export function SettingsSheet({ visible, soundEnabled, onToggleSound, onClose }: SettingsSheetProps) {
  return (
    <Modal visible={visible} transparent animationType="fade" onRequestClose={onClose}>
      <Pressable style={styles.backdrop} onPress={onClose}>
        <Pressable style={styles.sheet} onPress={(e) => e.stopPropagation()}>
          <View style={styles.row}>
            <Text style={styles.label}>Sound</Text>
            <Switch
              value={soundEnabled}
              onValueChange={onToggleSound}
              trackColor={{ false: 'rgba(255,255,255,0.15)', true: '#4FD1C5' }}
              thumbColor="#F5F5F7"
            />
          </View>
        </Pressable>
      </Pressable>
    </Modal>
  );
}

const styles = StyleSheet.create({
  backdrop: {
    flex: 1,
    backgroundColor: 'rgba(0,0,0,0.5)',
    justifyContent: 'flex-end',
  },
  sheet: {
    backgroundColor: '#161619',
    borderTopLeftRadius: 20,
    borderTopRightRadius: 20,
    paddingHorizontal: 24,
    paddingTop: 20,
    paddingBottom: 40,
  },
  row: {
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'space-between',
  },
  label: {
    color: '#F5F5F7',
    fontSize: 17,
    fontWeight: '400',
  },
});
