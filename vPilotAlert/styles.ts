import { StyleSheet } from 'react-native';

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
  dataRow: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
    paddingVertical: 8,
    borderBottomWidth: 1,
    borderBottomColor: '#eee',
  },
  dataLabel: {
    fontWeight: '600',
    fontSize: 16,
    color: '#333',
    flex: 1,
  },
  dataValue: {
    fontSize: 16,
    color: '#222',
    flex: 1,
    textAlign: 'right',
  },
  clearButton: {
    paddingVertical: 4,
    paddingHorizontal: 10,
    marginLeft: 12,
    borderWidth: 1,
    borderColor: '#ff3b30',
    borderRadius: 5,
  },
});

export default styles;
