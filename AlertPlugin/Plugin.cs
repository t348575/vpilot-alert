using RossCarlson.Vatsim.Vpilot.Plugins;
using RossCarlson.Vatsim.Vpilot.Plugins.Events;
using System;
using System.Net.Http;
using System.Text;
using System.Threading;
using System.IO;
using Newtonsoft.Json;

public class VpilotAlert : IPlugin
{
    private IBroker vPilot;
    public string Name { get; } = "VpilotAlert";
    private static readonly HttpClient client = new HttpClient();
    private Timer periodicTimer;
    private string baseUrl = "http://localhost:8080/vpilot-alert/api";

    public void Initialize(IBroker broker)
    {
        vPilot = broker;
        vPilot.PrivateMessageReceived += OnPrivateMessageReceived;
        vPilot.RadioMessageReceived += OnRadioMessageReceived;
        vPilot.SelcalAlertReceived += OnSelCalReceived;

        if (File.Exists("Plugins\\baseUrl.txt"))
        {
            baseUrl = File.ReadAllText("Plugins\\baseUrl.txt");
        }

        vPilot.PostDebugMessage($"Using baseUrl: {baseUrl}");

        periodicTimer = new Timer(PeriodicCheck, null, TimeSpan.Zero, TimeSpan.FromSeconds(5));
        vPilot.PostDebugMessage("vPilotAlert initialized.");
    }

    private async void PeriodicCheck(object state)
    {
        try
        {
            string url = $"{baseUrl}/connection-status";
            HttpResponseMessage response = await client.GetAsync(url);

            if (response.IsSuccessStatusCode)
            {
                string responseContent = await response.Content.ReadAsStringAsync();
                bool shouldDisconnect = JsonConvert.DeserializeObject<bool>(responseContent);

                if (!shouldDisconnect)
                {
                    vPilot.PostDebugMessage("Disconnect condition met. Disconnecting from vPilot...");
                    vPilot.RequestDisconnect();
                }
            }
            else
            {
                vPilot.PostDebugMessage($"Failed to check disconnect condition. Status code: {response.StatusCode}");
            }
        }
        catch (Exception ex)
        {
            vPilot.PostDebugMessage($"Error during periodic check: {ex.Message}");
        }
    }

    private async void SendRequest(string url, object payload_object)
    {
        try
        {
            string payload_string = JsonConvert.SerializeObject(payload_object);
            var content = new StringContent(payload_string, Encoding.UTF8, "application/json");
            var response = await client.PostAsync($"{baseUrl}{url}", content);

            if (!response.IsSuccessStatusCode)
            {
                vPilot.PostDebugMessage($"Failed to send message. Status code: {response.StatusCode}");
                string errorContent = await response.Content.ReadAsStringAsync();
                vPilot.PostDebugMessage($"Error: {errorContent}");
            }
        } catch (Exception ex)
        {
            vPilot.PostDebugMessage($"An error occurred sending request: {ex.Message}");
        }
    }

    private void OnPrivateMessageReceived(object sender, PrivateMessageReceivedEventArgs e)
    {
        SendRequest("/private-message", new { from = e.From, message = e.Message });
    }

    private void OnRadioMessageReceived(object sender, RadioMessageReceivedEventArgs e)
    {
        SendRequest("/radio-message", new { frequencies = e.Frequencies, from = e.From, message = e.Message });
    }

    private void OnSelCalReceived(object sender, SelcalAlertReceivedEventArgs e)
    {
        SendRequest("/selcal", new { frequencies = e.Frequencies, from = e.From });
    }
}
