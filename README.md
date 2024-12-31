<h1 align="center">vpilot-alert <img alt="Views" src="https://lambda.348575.xyz/repo-view-counter?repo=vpilot-alert"/></h1>

A server, [vPilot](https://vpilot.rosscarlson.dev/) plugin, and android app to play an alarm when you get message in vPilot.

## Usage
1. Grab the latest release zip [here](https://github.com/t348575/twitch-points-miner/releases) and extract the contents.
2. Copy `AlertPlugin.dll` to your vPilot plugin folder, usually here: `C:\Users\<your username>\AppData\Local\vPilot\Plugins`.
    * If you want to have external access (with a domain name for example) create a file `baseUrl.txt` in the same folder as the plugin, the contents being the base url to access your server, default is `http://localhost:8080/vpilot-alert/api`
3. Install the APK on your device, and configure the URL to access the server in the settings page, ie. the domain name or IP of the machine running the server.
    * Be sure to select an alarm sound as well
4. Open and connect vPilot.
5. Run the server from CMD of powershell, passing a `--callsign` argument to it, eg `./vpilot-alert.exe --callsign DHL145`

**Important note:** Once an alarm is triggered, press the `Stop Alarm` button to stop it. If the alarm is not stopped within 3 minutes, vPilot will automatically disconnect.