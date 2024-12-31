import {AppRegistry} from 'react-native';
import App from './App';
import {name as appName} from './app.json';
import messaging from '@react-native-firebase/messaging';
import { NativeModules } from 'react-native';
import AsyncStorage from '@react-native-async-storage/async-storage';
const { AlarmSounds, } = NativeModules;

messaging().setBackgroundMessageHandler(async (remoteMessage) => {
    const selectedSoundUri = await AsyncStorage.getItem('selectedSound');
    if (selectedSoundUri) {
        AlarmSounds.playSound(selectedSoundUri);
    }
});

AppRegistry.registerComponent(appName, () => App);
