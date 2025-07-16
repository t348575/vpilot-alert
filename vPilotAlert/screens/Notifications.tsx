import React, {useEffect, useState} from 'react';
import {
  View,
  Text,
  FlatList,
  TouchableOpacity,
  ActivityIndicator,
} from 'react-native';
import Toast from 'react-native-toast-message';
import {RefreshCw, Trash2} from 'lucide-react-native';
import {useBaseURL} from '../contexts/BaseURL';
import styles from '../styles';

const NotificationsScreen = () => {
  const {baseURL} = useBaseURL();
  const [notifications, setNotifications] = useState([]);
  const [loading, setLoading] = useState(true);
  const [refreshing, setRefreshing] = useState(false);

  const fetchNotifications = async () => {
    setLoading(true);
    try {
      const res = await fetch(`${baseURL}/notifications`);
      const data = await res.json();
      setNotifications(data);
    } catch (err) {
      console.error(err);
    } finally {
      setLoading(false);
    }
  };

  const refreshNotifications = async () => {
    setRefreshing(true);
    await fetchNotifications();
    setRefreshing(false);
  };

  const clearNotifications = async () => {
    try {
      await fetch(`${baseURL}/notifications`, {method: 'DELETE'});
      setNotifications([]);
      Toast.show({type: 'success', text1: 'Notifications cleared'});
    } catch (error) {
      console.error(error);
      Toast.show({
        type: 'error',
        text1: 'Error',
        text2: 'Failed to clear notifications',
      });
    }
  };

  useEffect(() => {
    fetchNotifications();
  }, []);

  const renderNotification = ({item}) => (
    <View style={styles.notificationCard}>
      <Text style={styles.notificationTitle}>{item.type}</Text>
      <Text>{item.message}</Text>
      <Text style={styles.notificationTimestamp}>{item.timestamp}</Text>
    </View>
  );

  return (
    <View style={styles.container}>
      <View style={styles.header}>
        <Text style={styles.title}>Notifications</Text>
        <TouchableOpacity
          onPress={refreshNotifications}
          style={styles.refreshButton}>
          <RefreshCw size={24} color="#007bff" />
        </TouchableOpacity>
        <TouchableOpacity
          onPress={clearNotifications}
          style={styles.clearButton}>
          <Trash2 size={24} color="#ff3b30" />
        </TouchableOpacity>
      </View>
      {loading ? (
        <ActivityIndicator size="large" color="#007bff" />
      ) : notifications.length > 0 ? (
        <FlatList
          data={notifications}
          keyExtractor={(item, idx) => idx.toString()}
          renderItem={renderNotification}
          refreshing={refreshing}
          onRefresh={refreshNotifications}
        />
      ) : (
        <Text>No notifications available.</Text>
      )}
    </View>
  );
};

export default NotificationsScreen;
