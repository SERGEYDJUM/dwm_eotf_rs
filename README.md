# About

When HDR is enabled, almost all SDR content in Windows looks wrong and washed out, even it's own UI. `dwm_eotf_rs` solves this problem by replacing Desktop Window Manager's piecewise sRGB transfer function with gamma curve[<sup>1<sup>](https://github.com/dylanraga/win11hdr-srgb-to-gamma2.2-icm). It does that by reading memory of the loaded `dwmcore.dll` module, patching shaders that contain "incorrect" math and writing it back.

`dwm_eotf_rs` is a rewrite (not a fork) of `dwm_eotf`[<sup>2<sup>](https://github.com/ledoge/dwm_eotf). This one provides fixes, improvements and additional features, such as system tray controls, Multi-Plane Overlay (MPO) toggle, autostart and even shader dumping. It's more reliable as well, as it does not require multiple retries for it to work and it's brightness scaling feature works much better.

**You do not need to disable/revert the patch (or restart DWM) when playing HDR games or videos. It only affects DWM composed SDR content!**

# Usage

## Quick Start

### Step 1

Download or compile the executable and put it somewhere safe.

### Step 2a - Compatibility Mode

If you do not plan to constantly switch Windows' HDR on and off, run the following commands as admin:
```
.\dwm_eotf_rs.exe -c
.\dwm_eotf_rs.exe -c schedule
```
The first one runs the app in Compatibility Mode and the second creates the task that runs on logon.

### Step 2b - Tray Mode

If you do plan to disable HDR often (with a Win + Alt + B shortcut, for example), dwm_eotf_rs has some great features for that use case. 
Launch the app, wait a bit (default `-w` is 5 seconds), then click tray icon and check the "Autostart" item, so that the app can start itself on logon.

Do note that most CLI flags and arguments from below do not have analogues in UI, but will work in Tray Mode and the startup task creation/update will respect them.

## Help Output
```
Patches DWM's shaders to use proper EOTF (gamma)

Usage: dwm_eotf_rs.exe [OPTIONS] [GAMMA] [COMMAND]

Commands:
  dump        Dumps DWM's original shaders as DXBC
  restore     Restores original sRGB EOTF and enables Multi-Plane Overlay
  schedule    Creates a task ('dwm_eotf_rs') that runs the app on user logon
  unschedule  Removes the startup task from Task Scheduler
  help        Print this message or the help of the given subcommand(s)

Arguments:
  [GAMMA]  Exponent to use during EOTF patching [default: 2.2]

Options:
  -c, --compatibility-mode       Patch DWM and exit (disables tray mode)
  -m, --mpo-state <MPO_STATE>    Optional Multi-Plane Overlay toggle (set to false to prevent some apps from bypassing DWM) [possible values: true, false]
  -n, --nits <NITS>              Optional target SDR brightness in nits (depends on display setup, see ReadMe)
  -i, --ignore-whitelist         Patch every shader that contains sRGB EOTF or alpha correction patterns
  -s, --skip-patching            Prevent automatic patching on app start (tray mode only)
  -w, --wait-time <WAIT_TIME>    Delay (in seconds) before patching on start (tray mode only) [default: 5]
      --brightness <BRIGHTNESS>  Optional brightness multiplier (legacy dwm_eotf compatible factor)
      --no-alpha-fix             Disable alpha correction fix when increasing brightness (see ReadMe)
  -h, --help                     Print help
  -V, --version                  Print version
```

## Tray Mode
By default, the app runs in system tray, where you can toggle patch as needed as well as select a gamma value (2.0/2.2/2.4/[GAMMA]).

When it launches, it will wait a few seconds (specified by `-w` option) before inital patching to avoid problems in some edge cases.

The app listens to Windows' WM_DISPLAYCHANGE messages to toggle the patch when HDR is enabled/disabled.

|||
|---------------------|---------------------|
|![](.assets/on.png)|![](.assets/off.png)|

## Compatibility Mode
When supplied with the `-c` flag, `dwm_eotf_rs` works like a simple console app - it patches DWM and exits.

![](.assets/compat.png)

## Startup

The app can register itself to run automatically when user logs in, using the Windows Task Scheduler (task `\<PC_NAME>\<USER_NAME>\dwm_eotf_rs`).

The `-c schedule` argument combination schedules `dwm_eotf_rs` to autostart in Compatibility Mode. **This is the intended way to use the app.**

In tray mode, the context menu includes an "Autostart" checkable item that toggles the startup task on or off. If the gamma value is changed while autostart is on, the task is updated to use the new gamma.

## Brightness Control

It's possible to set a desired paper white brightness in nits using the `-n` option. `dwm_eotf_rs` takes the highest SDR content brightness value of all your active HDR displays (as set in Windows Settings) and calculates the scaling factor for EOTF. The program additionally patches "AlphaCorrectSDR", "BoostSDRLuminance" and similar shaders to fix UI transparancy issues introduced by brightness boosting.

The `--brightness` option allows to set a brightness multiplier directly, but uses the math compatible with original `dwm_eotf` and might not be interpreted in terms of nits.

## Multi-Plane Overlay

Some apps and most games use the Multi-Plane Overlay (MPO) GPU feature to render their UI directly (so called "Independent Flip"), bypassing DWM and it's shaders (patched or otherwise). Most Electron and NW.js apps (e.g. VS Code, RPGM), Unity games, Chromium-based browsers and a lot of others use MPO.

`dwm_eotf_rs` will disable/enable MPO via registry when `-m false`/`-m true` is provided. There is also a relevant toggle in Tray Mode.

Do note that some apps and games will force Independent Flip if it's fullscreen anyway (when nothing obscures the app's window, to be more precise). Disabling MPO might also reduce performance of games.

## Whitelist
By default, `dwm_eotf_rs` will patch a predefined set of shaders (and an extra set when using `--nits`/`--brightness` without `--no-alpha-fix`). It should cover most use cases, but it's possible to patch all shaders with same patterns with an `-i` flag.

## Shader Dumping
The app can dump DWM's shaders as DXBC files for research purposes. DXBC files can be disassembled using 3Dmigoto's `cmd_Decompiler` with `--disassemble-ms` flag.

These shaders are nested. There are 30 top-level shaders and hundreds inside. Use `--big-shaders` flag to dump only former.

# Known Issues
- Chromium, NW.js and Electron-based apps (Web browsers, VS Code, RPGM games, etc) also use incorrect curves and will switch back and forth between original and fixed look sometimes. 
  - Setting `--force-color-profile` option (same as `#force-color-profile` in browser flags) to `hdr10` or `scrgb-linear` will remove flicker, but the app will use it's own HDR implementation.
  - Alternatively, set that flag to `srgb` and disable MPO (see section on Multi-Plane Overlay above). This will fix flicker and allow `dwm_eotf_rs` to work at the cost of HDR.
- As mentioned above, some SDR games bypass DWM regardless of MPO settings (when in fullscreen), so the `dwm_eotf_rs` will not help. Special K is a great app with HDR Retrofit[<sup>3<sup>](https://wiki.special-k.info/en/HDR/Retrofit) feature that includes the fix for this issue. Alternatively, ReShade with Lilium's "SDR TRC Fix"[<sup>4<sup>](https://github.com/EndlesslyFlowering/ReShade_HDR_shaders) shader can be used instead.

# Acknowledgements
- Many thanks to [ledoge](https://github.com/ledoge) for original C implementation.
- Thanks to 3Dmigoto project for the disassembler/decompiler.
