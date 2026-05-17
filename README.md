# About
An alternative implementation of the same idea that is behind [dwm_eotf](https://github.com/ledoge/dwm_eotf). 

It has additional features, such as system tray controls, autostart and shader dumping. This version is also more reliable, as it does not require multiple tries for it to work (as far as I can tell).

`dwm_eotf_rs` works by reading memory of the loaded `dwmcore.dll` module, patching shaders that are responsible for incorrect SDR to HDR conversions there and writing it back.

# Usage

## Help Output
```
Patches DWM's shaders to use proper EOTF (gamma)

Usage: dwm_eotf_rs.exe [OPTIONS] [GAMMA] [COMMAND]

Commands:
  restore         Restores original sRGB EOTF (by restarting DWM)
  schedule        Creates a task ('dwm_eotf_rs') that runs the app with default options on user logon
  unschedule      Removes the startup task ('dwm_eotf_rs') from Task Scheduler
  unschedule-all  Removes the startup task ('dwm_eotf_rs') from Task Scheduler for all users
  dump            Dumps DWM's original shaders as DXBC
  help            Print this message or the help of the given subcommand(s)

Arguments:
  [GAMMA]  Exponent to use during EOTF patching [default: 2.2]

Options:
  -c, --compatibility-mode     Patches DWM and exits (disables tray mode)
  -s, --skip-patching          Prevents automatic patching on app start (tray mode)
  -w, --wait-time <WAIT_TIME>  Delay (in seconds) before automatic patching on app start (tray mode) [default: 5]
  -i, --ignore-whitelist       Patch every shader that contains sRGB EOTF patterns
  -h, --help                   Print help
  -V, --version                Print version
```

### Tray Mode
By default, the app runs in system tray, where you can toggle patch as needed as well as select gamma value (2.0/2.2/2.4/[GAMMA]).

When it launches, it will wait a few seconds before inital patching, supposedly to help with crashes.

|||
|---------------------|---------------------|
|![](.assets/on.png)|![](.assets/off.png)|

### Compatibility Mode
This mode turns `dwm_eotf_rs` into a simple console app - it patches DWM and exits.

![](.assets/compat.png)

### Startup

`dwm_eotf_rs` can register itself to run automatically in **tray mode** when user logs in, using the Windows Task Scheduler (task named `dwm_eotf_rs`).

The system tray context menu includes a **"Autostart"** checkable item that toggles the startup task on or off. If the gamma value is changed while autostart is on, the task is updated to use the new gamma.

None of the app's flags are set automatically there for now, but you can edit the task manually to use compat mode, for example.

### Whitelist
By default, `dwm_eotf_rs` will patch only 4 shaders selected by ledoge. I think this covers most use cases, but it's possible to patch all shaders with same patterns by using `--ignore-whitelist` flag.

### Shader Dumping
The app can dump DWM's shaders as DXBC files for research purposes.

These shaders are nested. There are 30 top-level shaders and hundreds of sub-shaders. Use `--big-shaders` flag to dump only former.

## Library

dwm_eotf_rs depends on `shader_patcher` library from this repository that can be used to implement patching of other apps.

# Known Issues
- Chromium-based apps (Web browsers, VS Code, etc) also use incorrect curves and will switch back and forth between original and fixed look sometimes. Setting `#force-color-profile` flag to `hdr10` or `scrgb-linear` will help somewhat.

# Acknowledgements
- Many thanks to [ledoge](https://github.com/ledoge) for original C implementation.
