# waybar-module-mpd

An MPD (Music Player Daemon) music monitoring module for Waybar written in Rust.

Built with an event-driven architecture using MPD's native socket protocol (`idle`) to provide instantaneous updates.

## ✨ Features

- **Event-driven updates** - Uses MPD's socket `idle` event notification instead of continuous polling.
- **Waybar integration** - Single-line JSON output with CSS classes (`playing`, `paused`, `stopped`) and rich tooltips.
- **Duration & Volume tooltip** - Displays song title, artist, album, track duration (`mm:ss` or `hh:mm:ss`), and MPD volume.
- **Configurable** - Custom format template strings, icons, MPD server host/port, and stopped state labels.
- **Pure Rust** - No external C dependencies or D-Bus daemon requirements.

## 📦 Installation

### From Source

Ensure you have Rust and Cargo installed:

```bash
git clone https://github.com/NVL4826/waybar-module-mpd.git
cd waybar-module-mpd
cargo build --release

# Copy to your PATH
cp target/release/waybar-module-mpd ~/.local/bin/
```

### Arch Linux (PKGBUILD)

```bash
git clone https://github.com/NVL4826/waybar-module-mpd.git
cd waybar-module-mpd/dist/arch
makepkg -si
```

## ⚙️ Configuration

### Basic Waybar Setup

Add the custom module to your Waybar configuration (`~/.config/waybar/config`):

```json
{
  "custom/music": {
    "format": "{}",
    "return-type": "json",
    "exec": "waybar-module-mpd",
    "on-click": "mpc toggle",
    "on-scroll-up": "mpc volume +5",
    "on-scroll-down": "mpc volume -5"
  }
}
```

Include it in your bar modules list:

```json
{
  "modules-left": ["custom/music", "..."]
}
```

### CLI Options

```bash
waybar-module-mpd [OPTIONS]
```

| Option | Description | Default |
|---|---|---|
| `-H, --host <HOST>` | MPD server host (can use `MPD_HOST` env) | `127.0.0.1` |
| `-p, --port <PORT>` | MPD server port (can use `MPD_PORT` env) | `6600` |
| `-f, --format <TEMPLATE>` | Format template string (see placeholders below) | `[ %icon% ] %artist% - %title%` |
| `--play-icon <ICON>` | Play state icon | `` |
| `--pause-icon <ICON>` | Pause state icon | `` |
| `--stopped-icon <ICON>` | Stopped state icon | `` |
| `-s, --stopped-label <TEXT>` | Label displayed when MPD is stopped or offline | ` mpd` |
| `--debug` | Enable debug logging | `false` |
| `-h, --help` | Print help message | |
| `-V, --version` | Print version | |

### Format String Placeholders

You can customize the text template using `-f` / `--format`:

- `%icon%` - Current playback state icon (`play_icon`, `pause_icon`, or `stopped_icon`)
- `%title%` - Song title (falls back to filename if title tag is absent)
- `%artist%` - Artist name
- `%album%` - Album name
- `%duration%` - Total track duration (`mm:ss` or `hh:mm:ss`)
- `%volume%` - Current volume percentage (e.g. `85%`)

**Example:**
```bash
waybar-module-mpd --format "🎵 %artist% - %title% (%volume%)" --play-icon "▶" --pause-icon "⏸"
```

## 🎨 Styling

The module provides CSS classes for theming in your Waybar stylesheet (`~/.config/waybar/style.css`):

```css
#custom-music {
  padding: 0 10px;
  margin: 0 5px;
  border-radius: 8px;
}

#custom-music.playing {
  color: #a6e3a1;
  background: #1e1e2e;
}

#custom-music.paused {
  color: #f9e2af;
  background: #1e1e2e;
}

#custom-music.stopped {
  color: #6c7086;
  background: #1e1e2e;
}
```

## 🔧 Troubleshooting

Logs are written to:
```bash
~/.cache/waybar-module-mpd/app.log
```

Run with `--debug` to enable verbose logging when diagnosing connection issues with your MPD instance:
```bash
waybar-module-mpd --debug
```


