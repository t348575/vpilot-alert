import React, {useEffect} from 'react';
import {NavigationContainer} from '@react-navigation/native';
import {createBottomTabNavigator} from '@react-navigation/bottom-tabs';
import {Bell, Music, RefreshCw, Settings} from 'lucide-react-native';
import Toast from 'react-native-toast-message';

import {BaseURLProvider} from './contexts/BaseURL';
import AlarmSoundsScreen from './screens/AlarmSounds';
import NotificationsScreen from './screens/Notifications';
import LiveDataScreen from './screens/LiveData';
import SettingsScreen from './screens/Settings';
import requestPermissions from './requestPermissions';

const Tab = createBottomTabNavigator();

const App = () => {
  useEffect(() => {
    requestPermissions();
  }, []);

  return (
    <BaseURLProvider>
      <NavigationContainer>
        <Tab.Navigator
          screenOptions={({route}) => ({
            tabBarIcon: ({color, size}) => {
              let IconComponent;
              switch (route.name) {
                case 'Alarm Sounds':
                  IconComponent = Music;
                  break;
                case 'Notifications':
                  IconComponent = Bell;
                  break;
                case 'Live Data':
                  IconComponent = RefreshCw;
                  break;
                case 'Settings':
                  IconComponent = Settings;
                  break;
              }
              return IconComponent ? (
                <IconComponent color={color} size={size} />
              ) : null;
            },
            tabBarActiveTintColor: 'blue',
            tabBarInactiveTintColor: 'gray',
          })}>
          <Tab.Screen name="Alarm Sounds" component={AlarmSoundsScreen} />
          <Tab.Screen name="Live Data" component={LiveDataScreen} />
          <Tab.Screen name="Notifications" component={NotificationsScreen} />
          <Tab.Screen name="Settings" component={SettingsScreen} />
        </Tab.Navigator>
      </NavigationContainer>
      <Toast />
    </BaseURLProvider>
  );
};

export default App;
