# Testing Guide

## HackRF Detection Feature

### What's Fixed
The app now properly detects if a HackRF is connected before allowing RF transmission.

### Testing Without HackRF

1. **Start the app**:
   ```bash
   cd /home/z3d/Documents/repo/uapwhistle
   cargo run --release
   ```

2. **Navigate to RF section**:
   - Use arrow keys to scroll down to the HackRF section
   - You should see: `  RF Enable              [OFF] ✗ Not Found`

3. **Try to enable RF**:
   - Press `Space` on the "RF Enable" line
   - The RF worker will detect no device
   - It will automatically disable itself
   - Status will show `[OFF] ✗ Not Found`
   - You'll see error message in terminal: "HackRF not detected. Please connect HackRF and try again."

4. **Expected behavior**:
   - RF stays OFF even if you press Space
   - No hackrf_transfer process spawned
   - No error spam (checked every 5 seconds, then waits 2 seconds before retry)

### Testing With HackRF

1. **Connect HackRF**:
   - Plug in HackRF One via USB
   - Wait ~5 seconds for detection

2. **Start the app**:
   ```bash
   cargo run --release
   ```

3. **Check status**:
   - Navigate to RF Enable line
   - Should show: `  RF Enable              [OFF] ✓ Detected`

4. **Enable transmission**:
   - Press `Space` on RF Enable line
   - Status changes to: `[ON] ✓ Detected`
   - hackrf_transfer process starts
   - Transmission begins at displayed frequency

5. **Disconnect while running**:
   - Unplug HackRF while RF is ON
   - Within 5 seconds, detection updates to `✗ Not Found`
   - RF automatically disables
   - Process stops cleanly

## Being Selection Feature

### Testing Focus 10 (Theta)

1. **Load preset**:
   ```bash
   cargo run --release
   # Press 'l' to open preset browser
   # Select 'focus_10_mind_awake.json'
   # Press Enter
   ```

2. **Verify**:
   - Being Type shows: `[🧠 Focus 10 (Theta 4.1Hz)]`
   - Neural Coherence Volume shows: `4.1Hz Theta (meditation)`
   - All UAP channels show: `[MUTED - Human Mode]`
   - Only hear binaural beat tones (no chirps, no Schumann)

3. **Adjust volume**:
   - Navigate to "Neural Coherence Volume"
   - Use Left/Right arrows to adjust
   - Volume changes for binaural beats only

### Testing Focus 12 (Delta)

1. **Load preset**:
   ```bash
   # Press 'l' → select 'focus_12_expanded.json' → Enter
   ```

2. **Verify**:
   - Being Type shows: `[🌌 Focus 12 (Delta 1.5Hz)]`
   - Neural Coherence Volume shows: `1.5Hz Delta (deep)`
   - UAP channels muted
   - Very low, deep binaural beat (use headphones!)

### Testing UAP Mode

1. **Load UAP preset**:
   ```bash
   # Press 'l' → select 'uap_default.json' → Enter
   ```

2. **Verify**:
   - Being Type shows: `[🛸 UAP Mode]`
   - UAP channels active (no [MUTED] tags)
   - Hear Schumann carrier, chirps, harmonics
   - No binaural beats

### Testing Custom Mode

1. **Cycle to Custom**:
   - Navigate to "Being Type"
   - Press Right arrow 3 times: UAP → Focus10 → Focus12 → Custom
   - Shows: `[⚙️ Custom Human]`

2. **Adjust binaural beat**:
   - Navigate to "Binaural Beat Adjust"
   - Use Left/Right arrows to change frequency (0.1-30 Hz)
   - Brainwave state updates in real-time
   - Examples:
     - 2.0 Hz = Delta (deep)
     - 6.0 Hz = Theta (meditation)
     - 10.0 Hz = Alpha (relaxed)
     - 15.0 Hz = Beta (alert)

## Visual UI Test

### Expected Layout

```
Master Volume                  [||||||||||||........] 60%

╔═══ BEING SELECTION ═══╗
  Being Type                   [🧠 Focus 10 (Theta 4.1Hz)]
  Neural Coherence Volume             [||||||||||||||||....] 80% | 4.1Hz Theta (meditation)
  Binaural Beat Adjust         [Preset Mode] - Switch to Custom to adjust
╚═══════════════════════╝

╔═══ UAP FREQUENCIES ═══╗
  7.83Hz Carrier               [....................] 0% [SchumannAM] [MUTED - Human Mode]
  528Hz Harmonic               [....................] 0% [Sine] [MUTED - Human Mode]
  17kHz Ultrasonic Ping        [....................] 0% [Sine] [MUTED - Human Mode]
  Organic Chirps               [....................] 0% [OrganicChirp] [MUTED - Human Mode]
  432Hz Ambient Pad            [....................] 0% [Sine] [MUTED - Human Mode]
  Breath Layer                 [....................] 0% [LfoBreathing] [MUTED - Human Mode]
╚═══════════════════════╝

╔═══ HACKRF TRANSMIT ═══╗
  RF Enable                    [OFF] ✗ Not Found
  RF Frequency                 [1420.4 MHz] Mode: WBFM
  RF Gain (VGA)                [30 dB]
╚═══════════════════════╝
```

### Box Drawing Characters
- Should display cleanly (╔ ╗ ╚ ╝ ═)
- If garbled, terminal may not support UTF-8
- Try: `export LANG=en_US.UTF-8` before running

## Common Issues

### "No sound in Focus 10/12"
- Check Master Volume (should be 50-60%)
- Check Neural Coherence Volume (should be 70-80%)
- **Must use headphones** for binaural beats to work
- Increase volume gradually

### "Still hear chirps in Human mode"
- Reload preset with 'l' key
- Check that Being Type shows 🧠 or 🌌 (not 🛸)
- UAP channels should show `[MUTED - Human Mode]`

### "HackRF shows detected but won't transmit"
- Check USB connection
- Run `hackrf_info` in terminal to verify
- Check permissions: `sudo usermod -a -G plugdev $USER`
- Logout/login after adding to group

### "RF stays ON without device"
- This is now fixed - should auto-disable
- If persists, check that rf.rs includes detection code
- Rebuild: `cargo build --release`

## Performance Notes

- **CPU usage**: ~5-10% on modern CPU
- **HackRF detection**: Checks every 5 seconds (minimal overhead)
- **Audio latency**: <10ms typical
- **RF startup time**: ~1-2 seconds when enabling
