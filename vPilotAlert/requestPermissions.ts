import messaging from '@react-native-firebase/messaging';
import { PermissionsAndroid, Platform } from 'react-native';

const requestPermissions = async () => {
  await messaging().registerDeviceForRemoteMessages();
  await messaging().requestPermission();

  if (Platform.OS === 'android') {
    await PermissionsAndroid.request(PermissionsAndroid.PERMISSIONS.POST_NOTIFICATIONS);
  }
};
export default requestPermissions;
