<div align="center">
<img src="https://github.com/catilgrass/dumb-voice-protocol/blob/master/icon/icon.ico" width="30%" />
</div>

<h1 align="center">
  Dumb Voice Protocol
</h1>

## Introduction

A "dumb" voice input terminal based on [vtx-engine](https://github.com/keathmilligan/vtx-engine) and [Whisper](https://github.com/openai/whisper) — reads from microphone, transcribes to text, sends it to your game or tool.

Supports multiple output methods: stdout / stderr / TCP / UDP / UDP broadcast / IPC, suitable for various scenarios.

## Usage

1. Go to [Releases](https://github.com/catilgrass/dumb-voice-protocol/releases) and download the latest version
2. Unzip, edit `dmvop.toml` to configure
3. Double-click `dmvop.exe` and start speaking

## Usage (CLI)

1. Go to [Releases](https://github.com/catilgrass/dumb-voice-protocol/releases) and download
2. Open a terminal, run `dmvop -h` to see options
3. Speak into the microphone

Common examples:

```
dmvop                                           # Load dmvop.toml from current directory
dmvop --device="My Mic" -O=stdout               # Voice → terminal
dmvop --device="My Mic" -O=tcp -O=stdout        # Voice → game + terminal
dmvop --device="My Mic" -O=udp-broadcast        # Voice → LAN broadcast

dmvop --list-devices                            # List microphone devices
dmvop --download-model=small                    # Download whisper model
```

## Game Engine Binding

### 1. Unity

Place `binding\unity\DMVOPListener.cs` into your Unity project's `Assets` folder and it's ready to use.

### 2. Unreal

1. Build the binding plugin with the following command:

```powershell
.\binding\unreal\build-plugin.ps1 -UE "F:\YourUnrealEngine\UE_5.x"
```

2. Copy the `binding\unreal\build\DMVOPBridge` directory to your project or engine's `Plugins` folder.
3. Enable the `DMVOPBridge` plugin in the Unreal Editor.

## License

Under WTFPL
