import React, {
  createContext,
  useContext,
  useEffect,
  useRef,
  useState,
} from 'react';
import AsyncStorage from '@react-native-async-storage/async-storage';
import messaging from '@react-native-firebase/messaging';
import {View, ActivityIndicator} from 'react-native';
import Toast from 'react-native-toast-message';
import requestPermissions from '../requestPermissions';

interface BaseURLContextProps {
  baseURL: string | null;
  updateBaseURL: (newURL: string) => void;
}

export const BaseURLContext = createContext<BaseURLContextProps>({
  baseURL: '',
  updateBaseURL: () => {},
});

export const BaseURLProvider: React.FC<{children: React.ReactNode}> = ({
  children,
}) => {
  const [baseURL, setBaseURL] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const wasConnected = useRef(false);

  useEffect(() => {
    const loadBaseURL = async () => {
      try {
        const savedBaseURL = await AsyncStorage.getItem('baseURL');
        if (savedBaseURL) {
          setBaseURL(savedBaseURL);
        } else {
          setBaseURL('http://localhost:8080/vpilot-alert/api');
        }
        requestPermissions();
      } catch (error) {
        console.error('Failed to load baseURL:', error);
      }
      setLoading(false);
    };
    loadBaseURL();
  }, []);

  useEffect(() => {
    const checkConnection = async () => {
      if (!baseURL) return;
      try {
        const response = await fetch(`${baseURL}/notifications`);
        if (response.ok) {
          if (!wasConnected.current) {
            Toast.show({
              type: 'success',
              text1: 'Connected',
              text2: 'Successfully connected to the server!',
            });
            wasConnected.current = true;
          }
        } else {
          throw new Error();
        }
      } catch {
        wasConnected.current = false;
      }
    };
    checkConnection();
  }, [baseURL]);

  const updateBaseURL = async (newURL: string) => {
    setBaseURL(newURL);
    await AsyncStorage.setItem('baseURL', newURL);
    try {
      const response = await fetch(`${newURL}/notifications`);
      if (!response.ok) {
        throw new Error(
          `Unable to communicate with API, got: ${response.status}`,
        );
      }
      await response.json();
      Toast.show({
        type: 'success',
        text1: 'Connected',
        text2: 'Successfully connected to the server!',
      });
      wasConnected.current = true;
    } catch (error) {
      Toast.show({
        type: 'error',
        text1: 'Error',
        text2: `Unable to communicate with API: ${error}`,
      });
      wasConnected.current = false;
      return;
    }
  };

  useEffect(() => {
    if (!loading && baseURL) {
      const setFcmToken = async () => {
        const token = await messaging().getToken();
        await fetch(`${baseURL}/fcm-token`, {
          method: 'POST',
          headers: {'Content-Type': 'application/json'},
          body: JSON.stringify({token}),
        });
      };
      setFcmToken();
    }
  }, [loading, baseURL]);

  if (loading) {
    return (
      <View style={{flex: 1, justifyContent: 'center', alignItems: 'center'}}>
        <ActivityIndicator size="large" color="#007bff" />
      </View>
    );
  }

  return (
    <BaseURLContext.Provider value={{baseURL, updateBaseURL}}>
      {children}
    </BaseURLContext.Provider>
  );
};

export const useBaseURL = () => useContext(BaseURLContext);
