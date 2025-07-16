import React, {useEffect, useState} from 'react';
import {View, Text, FlatList, TouchableOpacity} from 'react-native';
import AsyncStorage from '@react-native-async-storage/async-storage';
import Sound from 'react-native-sound';
import {NativeModules} from 'react-native';
import {useBaseURL} from '../contexts/BaseURL';
import styles from '../styles';
const {AlarmSounds} = NativeModules;

interface SoundData {
  title: string;
  uri: string;
}

const AlarmSoundsScreen = () => {
  const {baseURL} = useBaseURL();
  const [sounds, setSounds] = useState<SoundData[]>([]);
  const [selectedSound, setSelectedSound] = useState<string | null>(null);
  const [currentSound, setCurrentSound] = useState<Sound | null>(null);

  useEffect(() => {
    const loadSelectedSound = async () => {
      const savedUri = await AsyncStorage.getItem('selectedSound');
      if (savedUri) setSelectedSound(savedUri);
    };
    loadSelectedSound();
  }, []);

  useEffect(() => {
    const fetchSounds = async () => {
      try {
        const result = await AlarmSounds.getAlarmSounds();
        setSounds(JSON.parse(result));
      } catch (error) {
        console.error('Error fetching sounds:', error);
      }
    };
    fetchSounds();
  }, []);

  const playSound = (uri: string) => {
    if (currentSound) {
      currentSound.stop(() => {
        currentSound.release();
      });
    }
    const sound = new Sound(uri, '', error => {
      if (error) {
        console.error('Failed to load the sound', error);
        return;
      }
      setCurrentSound(sound);
      sound.play(success => {
        if (!success) {
          console.error('Playback failed due to audio decoding errors');
        }
      });
      setTimeout(() => {
        sound.stop(() => {
          sound.release();
          setCurrentSound(null);
        });
      }, 5000);
    });
  };

  const selectSound = async (uri: string) => {
    setSelectedSound(uri);
    await AsyncStorage.setItem('selectedSound', uri);
  };

  const stopAlarm = async () => {
    AlarmSounds.stopSound();
    await fetch(`${baseURL}/alarm`, {method: 'DELETE'});
  };

  return (
    <View style={styles.container}>
      <FlatList
        data={sounds}
        keyExtractor={item => item.uri}
        renderItem={({item}) => (
          <TouchableOpacity
            style={styles.soundItem}
            onPress={() => {
              playSound(item.uri);
              selectSound(item.uri);
            }}>
            <Text>{item.title}</Text>
          </TouchableOpacity>
        )}
      />
      {selectedSound && (
        <View style={styles.selectedSound}>
          <Text>Selected Sound: {selectedSound}</Text>
        </View>
      )}
      <TouchableOpacity onPress={stopAlarm} style={styles.stopButton}>
        <Text style={styles.stopButtonText}>Stop Alarm</Text>
      </TouchableOpacity>
    </View>
  );
};

export default AlarmSoundsScreen;
