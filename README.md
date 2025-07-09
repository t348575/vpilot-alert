<h1 align="center">vpilot-alert <img alt="Views" src="https://lambda.348575.xyz/repo-view-counter?repo=vpilot-alert"/></h1>

A server, [vPilot](https://vpilot.rosscarlson.dev/) plugin, and an android app to play an alarm when you get message in vPilot (private message, callsign on radio, selcal). Also displays some route statistics, and does some primitve crash validation.

*__Note__: This application is not meant to allow AFK operations on Vatsim, outside Vatsim's CoC limit of 30mins in uncontrolled airspace.*

## Usage
1. Grab the latest release zip [here](https://github.com/t348575/vpilot-alert/releases) and extract the contents.
2. Copy `AlertPlugin.dll` to your vPilot plugin folder, usually here: `C:\Users\<your username>\AppData\Local\vPilot\Plugins`.
    * If you want to have external access (with a domain name for example) create a file `baseUrl.txt` in the same folder as the plugin, the contents being the base url to access your server, default is `http://localhost:8080/vpilot-alert/api`
3. Open and connect vPilot.
4. Run the server from CMD or powershell, passing a `--callsign` argument to it, as well as a navigraph navigation database eg `./vpilot-alert.exe --callsign DHL145 -n path_to_navdb`
5. Install the APK on your device, and configure the URL to access the server in the settings page, ie. the domain name or IP of the machine running the server.
    * Be sure to select an alarm sound, else no alarm is played
    * __Note:__ It is important to have the server running before opening the app, so that it can register itself for notifications with the server.
  
 * For debugging purposes, you can set the environment variable `LOG` to debug when running the server.

__Important note:__ Once an alarm is triggered, press the `Stop Alarm` button to stop it. If the alarm is not stopped within 3 minutes, a disconnect is triggered through vPilot.

## Crash detection parameters (in cruise)
* Aircraft route loops
* Aircraft position does not update for 3 minutes
* Aircraft drops out of RVSM (FL290)
* Ground speed below 300
* Route deviations more than 30nm