import React, { useEffect, useState } from 'react';
import { View, Text, TouchableOpacity, ActivityIndicator } from 'react-native';
import { useBaseURL } from '../contexts/BaseURL';
import styles from '../styles';

const DataRow = ({ label, value }: { label: string; value: React.ReactNode }) => (
  <View style={styles.dataRow}>
    <Text style={styles.dataLabel}>{label}</Text>
    <Text style={styles.dataValue}>{value}</Text>
  </View>
);

interface PilotInfo {
  callsign: string;
  latitude: number;
  longitude: number;
  altitude: number;
  groundspeed: number;
}

interface LiveData {
  leftover_route: string[];
  next_waypoint: string;
  prev_waypoint: string;
  route_deviation: number;
  route_progress: number;
  dist_next_wp: number;
  in_loop: boolean;
  stuck: boolean;
  pilot?: PilotInfo;
}

const LiveDataScreen = () => {
  const { baseURL } = useBaseURL();
  const [data, setData] = useState<LiveData | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [routeCollapsed, setRouteCollapsed] = useState(true);

  useEffect(() => {
    let isMounted = true;
    const fetchLiveData = async () => {
      setLoading(true);
      setError(null);
      try {
        const response = await fetch(`${baseURL}/stats`);
        if (!response.ok) throw new Error(`HTTP ${response.status}`);
        const result = await response.json();
        if (isMounted) setData(result);
      } catch (err) {
        if (isMounted) { setError('Failed to fetch live data.'); setData(null); }
      }
      if (isMounted) setLoading(false);
    };
    fetchLiveData();
    const interval = setInterval(fetchLiveData, 15000);
    return () => { isMounted = false; clearInterval(interval); };
  }, [baseURL]);

  return (
    <View style={styles.container}>
      <Text style={styles.title}>Live Data</Text>
      {loading ? (
        <ActivityIndicator size="large" color="#007bff" />
      ) : error ? (
        <Text style={{ color: 'red' }}>{error}</Text>
      ) : !data ? (
        <Text>No live data available.</Text>
      ) : (
        <View>
          <View style={{ marginBottom: 10 }}>
            <Text style={styles.dataLabel}>Leftover Route:</Text>
            {data.leftover_route && data.leftover_route.length > 4 ? (
              <TouchableOpacity
                onPress={() => setRouteCollapsed(!routeCollapsed)}
                style={{ paddingVertical: 6 }}
              >
                <Text
                  numberOfLines={routeCollapsed ? 1 : undefined}
                  ellipsizeMode="tail"
                  style={{ color: '#007bff' }}
                >
                  {data.leftover_route.join(' → ')}
                </Text>
                <Text style={{ color: '#888', fontSize: 12 }}>
                  {routeCollapsed ? 'Show more ▼' : 'Show less ▲'}
                </Text>
              </TouchableOpacity>
            ) : (
              <Text style={styles.dataValue}>
                {data.leftover_route ? data.leftover_route.join(' → ') : '-'}
              </Text>
            )}
          </View>
          <DataRow label="Next Waypoint:" value={data.next_waypoint ?? '-'} />
          <DataRow label="Previous Waypoint:" value={data.prev_waypoint ?? '-'} />
          <DataRow label="Route Deviation (NM):" value={data.route_deviation != null ? data.route_deviation.toFixed(2) : '-'} />
          <DataRow label="Route Progress (%):" value={data.route_progress != null ? `${data.route_progress.toFixed(1)}%` : '-'} />
          <DataRow label="Distance to Next WP (NM):" value={data.dist_next_wp != null ? data.dist_next_wp.toFixed(2) : '-'} />
          <DataRow label="In Loop:" value={data.in_loop ? 'Yes' : 'No'} />
          <DataRow label="Stuck:" value={data.stuck ? 'Yes' : 'No'} />
          <View style={{ marginTop: 18 }}>
            <Text style={styles.dataLabel}>Pilot Info:</Text>
          </View>
          <DataRow label="Callsign:" value={data.pilot?.callsign ?? '-'} />
          <DataRow label="Lat/Lon:" value={data.pilot ? `${data.pilot.latitude}, ${data.pilot.longitude}` : '-'} />
          <DataRow label="Altitude:" value={data.pilot?.altitude != null ? `${data.pilot.altitude} ft` : '-'} />
          <DataRow label="Ground Speed:" value={data.pilot?.groundspeed != null ? `${data.pilot.groundspeed} knots` : '-'} />
        </View>
      )}
    </View>
  );
};

export default LiveDataScreen;
