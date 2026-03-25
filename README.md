# About
This is a WIP rewrite of the great [dwm_eotf](https://github.com/ledoge/dwm_eotf) with some additional features. 

I also think this version is more stable, as it never required multiple attempts to apply the effect.

Both programs work by patching memory of the loaded `dwmcore.dll` module containing shaders that are responsible for incorrect SDR to HDR conversions.

# Usage

## Help Output
```
Patches DWM's shaders to use proper EOTF (gamma)

Usage: dwm_eotf_rs.exe [OPTIONS] [GAMMA]

Arguments:
  [GAMMA]  Gamma to use in patched EOTF [default: 2.2]

Options:
  -t, --tray-mode          The app will run in system tray until closed
  -p, --patch-immidiately  The tray mode will patch DWM at the start
  -r, --restore            Restores original EOTF by restarting the DWM
  -h, --help               Print help
  -V, --version            Print version
```

## Tray Mode
You can toggle the patch using a system tray icon, as well as select between preset gamma values (2.0/2.2/2.4).

## Library

dwm_eotf_rs depends on `shader_patcher` library from this repository that can be used to implement patching of other apps.

# Known Issues
- Chromium-based apps (Web browsers, VS Code, etc) also use incorrect curves and will switch back and forth between original and fixed look.

# Acknowledgements
- Many thanks to [ledoge](https://github.com/ledoge) for original C implementation.
