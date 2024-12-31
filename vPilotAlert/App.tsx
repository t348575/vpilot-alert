import React, { createContext, useContext, useEffect, useState } from 'react';
import { View, Text, FlatList, TouchableOpacity, StyleSheet, PermissionsAndroid, Platform, ActivityIndicator, TextInput, Button } from 'react-native';
import { NativeModules } from 'react-native';
import Sound from 'react-native-sound';
import AsyncStorage from '@react-native-async-storage/async-storage';
import messaging from '@react-native-firebase/messaging';
import { NavigationContainer } from "@react-navigation/native";
import { createBottomTabNavigator } from "@react-navigation/bottom-tabs";
import { Bell, Music, RefreshCw, Settings } from "lucide-react-native";
import './FirebaseConfig';

const { AlarmSounds } = NativeModules;

const BaseURLContext = createContext({ baseURL: "", updateBaseURL: (newURL: any) => {} });
const BaseURLProvider = ({ children }) => {
  const [baseURL, setBaseURL] = useState(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    const loadBaseURL = async () => {
      try {
        const savedBaseURL = await AsyncStorage.getItem("baseURL");
        if (savedBaseURL) {
          setBaseURL(savedBaseURL);
          console.log(`Loaded baseURL: ${savedBaseURL}`);
        } else {
          setBaseURL("http://localhost:8080/vpilot-alert/api");
        }
      } catch (error) {
        console.error("Failed to load baseURL:", error);
      }

      setLoading(false);
    };

    loadBaseURL();
  }, []);

  const updateBaseURL = async (newURL) => {
    setBaseURL(newURL);
    await AsyncStorage.setItem("baseURL", newURL);
  };

  useEffect(() => {
    if (!loading && baseURL) {
      const setFcmToken = async () => {
        console.log(baseURL)
        const token = await messaging().getToken();
        await fetch(`${baseURL}/fcm-token`, {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
          },
          body: JSON.stringify({ token }),
        });
      };
      setFcmToken();
    }
  }, [loading, baseURL]);

  if (loading) {
    return (
      <View style={{ flex: 1, justifyContent: "center", alignItems: "center" }}>
        <ActivityIndicator size="large" color="#007bff" />
      </View>
    );
  }

  return (
    <BaseURLContext.Provider value={{ baseURL, updateBaseURL }}>
      {children}
    </BaseURLContext.Provider>
  );
};
const useBaseURL = () => useContext(BaseURLContext);

const AlarmSoundsScreen = () => {
  const { baseURL } = useBaseURL();
  const [sounds, setSounds] = useState([]);
  const [selectedSound, setSelectedSound] = useState(null);
  const [currentSound, setCurrentSound] = useState(null);

  useEffect(() => {
    const loadSelectedSound = async () => {
      try {
        const savedUri = await AsyncStorage.getItem("selectedSound");
        if (savedUri) {
          setSelectedSound(savedUri);
        }
      } catch (error) {
        console.error("Failed to load saved sound:", error);
      }
    };

    loadSelectedSound();
  }, []);

  useEffect(() => {
    const fetchSounds = async () => {
      try {
        const result = await AlarmSounds.getAlarmSounds();
        const sounds = JSON.parse(result);
        setSounds(sounds);
      } catch (error) {
        console.error("Error fetching sounds:", error);
      }
    };
    fetchSounds();
  }, []);

  const playSound = (uri) => {
    if (currentSound) {
      currentSound.stop(() => {
        currentSound.release();
      });
    }

    const sound = new Sound(uri, "", (error) => {
      if (error) {
        console.error("Failed to load the sound", error);
        return;
      }

      setCurrentSound(sound);

      sound.play((success) => {
        if (!success) {
          console.error("Playback failed due to audio decoding errors");
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

  const selectSound = async (uri) => {
    setSelectedSound(uri);
    await AsyncStorage.setItem('selectedSound', uri);
  };

    const requestPermissions = async () => {
    const authStatus = await messaging().requestPermission();

    if (Platform.OS === 'android') {
      await PermissionsAndroid.request(PermissionsAndroid.PERMISSIONS.POST_NOTIFICATIONS);
    }
  };

  useEffect(() => {
    requestPermissions();

    messaging().setBackgroundMessageHandler(async (remoteMessage) => {
      await playAlarmSound();
    });

    const unsubscribe = messaging().onMessage(async (remoteMessage) => {
      await playAlarmSound();
    });

    return () => {
      unsubscribe();
    };
  }, []);

  const playAlarmSound = async () => {
    const selectedSoundUri = await AsyncStorage.getItem('selectedSound');
    if (selectedSoundUri) {
      AlarmSounds.playSound(selectedSoundUri);
    }
  };

  const stopAlarm = async () => {
    AlarmSounds.stopSound();
    await fetch(`${baseURL}/alarm`, { method: "DELETE" });
  };

  return (
    <View style={styles.container}>
      <FlatList
        data={sounds}
        keyExtractor={(item) => item.uri}
        renderItem={({ item }) => (
          <TouchableOpacity
            style={styles.soundItem}
            onPress={() => {
              playSound(item.uri);
              selectSound(item.uri);
            }}
          >
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

const NotificationsScreen = () => {
  const { baseURL } = useBaseURL();
  const [notifications, setNotifications] = useState([]);
  const [loading, setLoading] = useState(true);
  const [refreshing, setRefreshing] = useState(false);

  const fetchNotifications = async () => {
    try {
      setLoading(true);
      const response = await fetch(`${baseURL}/notifications`);
      const data = await response.json();
      setNotifications(data);
    } catch (error) {
      console.error("Error fetching notifications:", error);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchNotifications();
  }, []);

  const refreshNotifications = async () => {
    try {
      setRefreshing(true);
      const response = await fetch(`${baseURL}/notifications`);
      const data = await response.json();
      setNotifications(data);
    } catch (error) {
      console.error("Error refreshing notifications:", error);
    } finally {
      setRefreshing(false);
    }
  };

  const renderNotification = ({ item }) => (
    <View style={styles.notificationCard}>
      <Text style={styles.notificationTitle}>{item._type}</Text>
      <Text>{item.message}</Text>
      <Text style={styles.notificationTimestamp}>{item.timestamp}</Text>
    </View>
  );

  return (
    <View style={styles.container}>
      <View style={styles.header}>
        <Text style={styles.title}>Refresh</Text>
        <TouchableOpacity onPress={refreshNotifications} style={styles.refreshButton}>
          <RefreshCw size={24} color="#007bff" />
        </TouchableOpacity>
      </View>
      {loading ? (
        <ActivityIndicator size="large" color="#007bff" />
      ) : notifications.length > 0 ? (
        <FlatList
          data={notifications}
          keyExtractor={(item, index) => index.toString()}
          renderItem={renderNotification}
          refreshing={refreshing}
          onRefresh={refreshNotifications}
        />
      ) : (
        <Text style={styles.noNotificationsText}>No notifications available.</Text>
      )}
    </View>
  );
};

const SettingsScreen = () => {
  const { baseURL, updateBaseURL } = useBaseURL();
  const [newURL, setNewURL] = useState(baseURL);

  const saveNewURL = () => {
    updateBaseURL(newURL);
  };

  return (
    <View style={styles.container}>
      <Text style={styles.label}>Base URL:</Text>
      <TextInput
        style={styles.input}
        value={newURL}
        onChangeText={setNewURL}
        placeholder="Enter Base URL"
      />
      <Button title="Save" onPress={saveNewURL} />
    </View>
  );
};

const Tab = createBottomTabNavigator();

const App = () => {
  return (
    <BaseURLProvider>
      <NavigationContainer>
        <Tab.Navigator
          screenOptions={({ route }) => ({
            tabBarIcon: ({ color, size }) => {
              let IconComponent;

              if (route.name === "Alarm Sounds") {
                IconComponent = Music;
              } else if (route.name === "Notifications") {
                IconComponent = Bell;
              } else if (route.name === "Settings") {
                IconComponent = Settings;
              }

              return <IconComponent color={color} size={size} />;
            },
            tabBarActiveTintColor: "blue",
            tabBarInactiveTintColor: "gray",
          })}
        >
          <Tab.Screen name="Alarm Sounds" component={AlarmSoundsScreen} />
          <Tab.Screen name="Notifications" component={NotificationsScreen} />
          <Tab.Screen name="Settings" component={SettingsScreen} />
        </Tab.Navigator>
      </NavigationContainer>
    </BaseURLProvider>
  );
};

const styles = StyleSheet.create({
  container: {
    flex: 1,
    padding: 20,
    backgroundColor: "#fff",
  },
  header: {
    flexDirection: "row",
    justifyContent: "space-between",
    alignItems: "center",
    marginBottom: 16,
  },
  title: {
    fontSize: 18,
    fontWeight: "bold",
    marginBottom: 10,
  },
  label: {
    fontSize: 16,
    marginBottom: 8,
  },
  input: {
    borderWidth: 1,
    borderColor: "#ddd",
    padding: 8,
    borderRadius: 5,
    marginBottom: 16,
  },
  soundItem: {
    padding: 10,
    borderBottomWidth: 1,
    borderBottomColor: "#ddd",
  },
  selectedSound: {
    marginTop: 20,
    padding: 10,
    backgroundColor: "#f0f0f0",
    borderRadius: 5,
  },
  refreshButton: {
    padding: 8,
    borderWidth: 1,
    borderRadius: 5
  },
  notificationCard: {
    padding: 10,
    marginBottom: 10,
    backgroundColor: "#f9f9f9",
    borderRadius: 5,
    borderWidth: 1,
    borderColor: "#ddd",
  },
  notificationTitle: {
    fontSize: 16,
    fontWeight: "bold",
    marginBottom: 5,
  },
  notificationTimestamp: {
    fontSize: 12,
    color: "#666",
    marginTop: 5,
  },
  stopButton: {
    marginTop: 20,
    padding: 15,
    backgroundColor: 'red',
    borderRadius: 5,
  },
  stopButtonText: {
    color: '#fff',
    fontWeight: 'bold',
    textAlign: 'center',
  },
});

export default App;