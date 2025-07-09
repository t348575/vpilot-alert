import React, { useEffect, useState } from 'react';
import { View, Text, TextInput, Button, Switch } from 'react-native';
import Toast from 'react-native-toast-message';
import { useBaseURL } from '../contexts/BaseURL';
import styles from '../styles';

const SettingsScreen = () => {
  const { baseURL, updateBaseURL } = useBaseURL();
  const [newURL, setNewURL] = useState(baseURL || '');
  const [alertCrash, setAlertCrash] = useState(false);
  const [loadingCrash, setLoadingCrash] = useState(false);

  useEffect(() => {
    const fetchAlertCrash = async () => {
      const response = await fetch(`${baseURL}/alert_crashes`);
      const data = await response.json();
      setAlertCrash(data);
    }
    fetchAlertCrash();
  }, []);

  const saveNewURL = () => { updateBaseURL(newURL); };

  const toggleAlertCrash = async (value: boolean) => {
    setLoadingCrash(true);
    setAlertCrash(value);
    try {
      await fetch(`${baseURL}/alert_crashes/${value}`, { method: "POST" });
      Toast.show({
        type: "success",
        text1: "Alert crash toggled",
        text2: value ? "Crash alerts ON" : "Crash alerts OFF"
      });
    } catch (err) {
      Toast.show({
        type: "error",
        text1: "Error",
        text2: "Failed to toggle crash alerts"
      });
      setAlertCrash(!value);
    }
    setLoadingCrash(false);
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

      <View style={{ flexDirection: "row", alignItems: "center", marginTop: 30 }}>
        <Text style={{ flex: 1, fontSize: 16 }}>Alert crash</Text>
        <Switch
          value={alertCrash}
          onValueChange={toggleAlertCrash}
          disabled={loadingCrash}
        />
      </View>
    </View>
  );
};

export default SettingsScreen;
