import {AppRegistry, NativeModules} from 'react-native';
import messaging from '@react-native-firebase/messaging';
import AsyncStorage from '@react-native-async-storage/async-storage';
const {AlarmSounds} = NativeModules;

let baseURL = null;
async function playAlarm(remoteMessage) {
  if (remoteMessage.data.triggerAlarm === 'true' && !AlarmSounds.isPlaying()) {
    if (!baseURL) {
      baseURL = await AsyncStorage.getItem('baseURL');
    }
    await fetch(`${baseURL}/alarm`, {method: 'POST'});

    const selectedSoundUri = await AsyncStorage.getItem('selectedSound');
    if (selectedSoundUri) {
      AlarmSounds.playSound(selectedSoundUri);
    }
  }
}

messaging().setBackgroundMessageHandler(playAlarm);
messaging().onMessage(playAlarm);

import App from './App';
import {name as appName} from './app.json';

AppRegistry.registerComponent(appName, () => App);
