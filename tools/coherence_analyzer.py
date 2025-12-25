#!/usr/bin/env python3
"""
Hemi-Sync Audio Frequency Analyzer

This script analyzes Hemi-Sync audio files (Focus 10, Focus 12, etc.) to extract:
- Automatic section detection based on frequency changes
- Frequency analysis for left and right channels
- Binaural beat calculations
- Timeline mapping of frequency changes

Usage:
    python3 coherence_analyzer.py <audio_file.flac> [--output report.json]
"""

import numpy as np
import scipy.io.wavfile as wavfile
import scipy.signal as signal
from scipy.fft import rfft, rfftfreq
import json
import sys
import subprocess
import os
import tempfile
import argparse
from pathlib import Path
from typing import Dict, List, Tuple, Any


class CoherenceAnalyzer:
    """Analyzes Hemi-Sync audio files to extract frequency and section information."""

    def __init__(self, audio_file: str, chunk_duration: int = 30, overlap: int = 15):
        """
        Initialize the analyzer.

        Args:
            audio_file: Path to the audio file (FLAC, OGG, MP3, WAV, etc.)
            chunk_duration: Duration of each analysis chunk in seconds
            overlap: Overlap between chunks in seconds
        """
        self.audio_file = audio_file
        self.chunk_duration = chunk_duration
        self.overlap = overlap
        self.sample_rate: int = 0
        self.audio_data = None
        self.left_channel = None
        self.right_channel = None
        self.duration: float = 0.0

    def load_audio(self) -> None:
        """Load audio file and convert to WAV if necessary."""
        print(f"Loading audio file: {self.audio_file}")

        # Create temporary WAV file if input is not WAV
        if not self.audio_file.lower().endswith(".wav"):
            temp_wav = tempfile.NamedTemporaryFile(suffix=".wav", delete=False)
            temp_wav.close()

            # Convert to WAV using ffmpeg
            cmd = [
                "ffmpeg",
                "-i",
                self.audio_file,
                "-acodec",
                "pcm_s16le",
                "-ar",
                "44100",
                "-ac",
                "2",
                "-y",
                temp_wav.name,
            ]

            result = subprocess.run(cmd, capture_output=True, text=True)
            if result.returncode != 0:
                raise RuntimeError(f"FFmpeg conversion failed: {result.stderr}")

            wav_file = temp_wav.name
        else:
            wav_file = self.audio_file

        # Load WAV file
        self.sample_rate, self.audio_data = wavfile.read(wav_file)

        # Clean up temporary file
        if wav_file != self.audio_file:
            os.unlink(wav_file)

        # Ensure stereo
        if len(self.audio_data.shape) == 1:
            raise ValueError("Audio file must be stereo (2 channels)")

        # Split into left and right channels
        self.left_channel = self.audio_data[:, 0].astype(float)
        self.right_channel = self.audio_data[:, 1].astype(float)
        self.duration = len(self.audio_data) / self.sample_rate

        print(f"Loaded: {self.duration:.1f}s, {self.sample_rate}Hz, stereo")

    def analyze_frequency_spectrum(
        self, channel_data, start_sample: int, end_sample: int
    ) -> Dict[str, Any]:
        """
        Analyze frequency spectrum for a given audio segment.

        Args:
            channel_data: Audio data for one channel
            start_sample: Starting sample index
            end_sample: Ending sample index

        Returns:
            dict with dominant frequency and spectrum info
        """
        segment = channel_data[start_sample:end_sample]

        # Apply window function to reduce spectral leakage
        window = signal.windows.hann(len(segment))
        windowed = segment * window

        # Compute FFT
        fft_vals = rfft(windowed)
        fft_freq = rfftfreq(len(windowed), 1.0 / self.sample_rate)

        # Get magnitude spectrum
        magnitude = np.abs(fft_vals)

        # Focus on frequencies between 50 Hz and 1000 Hz (typical carrier range)
        freq_mask = (fft_freq >= 50) & (fft_freq <= 1000)
        freq_range = fft_freq[freq_mask]
        mag_range = magnitude[freq_mask]

        # Find dominant frequency
        if len(mag_range) > 0:
            peak_idx = np.argmax(mag_range)
            dominant_freq = freq_range[peak_idx]
            peak_magnitude = mag_range[peak_idx]

            # Find secondary peaks (for harmonic analysis)
            peaks, properties = signal.find_peaks(
                mag_range, height=peak_magnitude * 0.3
            )
            peak_freqs = freq_range[peaks]
            peak_mags = properties["peak_heights"]

            # Sort by magnitude
            sorted_indices = np.argsort(peak_mags)[::-1]
            top_peaks = [(peak_freqs[i], peak_mags[i]) for i in sorted_indices[:5]]

        else:
            dominant_freq = 0
            top_peaks = []

        return {
            "dominant_frequency": float(dominant_freq),
            "top_frequencies": [(float(f), float(m)) for f, m in top_peaks],
            "rms_amplitude": float(np.sqrt(np.mean(segment**2))),
        }

    def detect_sections(self) -> List[Dict[str, Any]]:
        """
        Detect different sections in the audio based on frequency changes.

        Analyzes the entire file to find distinct sections where frequencies
        remain relatively stable, then transitions to different frequencies.

        Returns:
            list of section dictionaries with start_time, end_time, and section_type
        """
        print("\nDetecting sections based on frequency changes...")

        # First pass: analyze frequency over time with smaller chunks
        window_size = int(
            self.sample_rate * 10
        )  # 10-second windows for frequency analysis
        hop_size = int(self.sample_rate * 5)  # 5-second hops

        freq_timeline: List[Dict[str, float]] = []
        times: List[float] = []

        for i in range(0, len(self.left_channel) - window_size, hop_size):
            # Analyze both channels
            left_analysis = self.analyze_frequency_spectrum(
                self.left_channel, i, i + window_size
            )
            right_analysis = self.analyze_frequency_spectrum(
                self.right_channel, i, i + window_size
            )

            binaural_beat = abs(
                left_analysis["dominant_frequency"]
                - right_analysis["dominant_frequency"]
            )

            freq_timeline.append(
                {
                    "time": i / self.sample_rate,
                    "left_freq": left_analysis["dominant_frequency"],
                    "right_freq": right_analysis["dominant_frequency"],
                    "binaural_beat": binaural_beat,
                    "amplitude": left_analysis["rms_amplitude"],
                }
            )
            times.append(i / self.sample_rate)

        if len(freq_timeline) == 0:
            print("Warning: Unable to analyze frequency timeline, using single section")
            return [
                {
                    "section_type": "full",
                    "start_time": 0.0,
                    "end_time": float(self.duration),
                    "duration": float(self.duration),
                    "avg_binaural_beat": 0.0,
                    "avg_left_freq": 0.0,
                }
            ]

        # Extract frequency series for change detection
        binaural_series = np.array([f["binaural_beat"] for f in freq_timeline])
        left_series = np.array([f["left_freq"] for f in freq_timeline])
        amplitude_series = np.array([f["amplitude"] for f in freq_timeline])

        # Smooth the frequency series to reduce noise
        if len(binaural_series) > 5:
            window_len = min(
                11,
                len(binaural_series)
                if len(binaural_series) % 2 == 1
                else len(binaural_series) - 1,
            )
            binaural_smooth = signal.savgol_filter(binaural_series, window_len, 3)
            left_smooth = signal.savgol_filter(left_series, window_len, 3)
        else:
            binaural_smooth = binaural_series
            left_smooth = left_series

        # Detect change points where frequency shifts significantly
        change_points: List[int] = [0]  # Start with beginning

        # Look for significant changes in binaural beat frequency
        for i in range(1, len(binaural_smooth)):
            # Check for significant change (more than 2 Hz difference)
            if abs(binaural_smooth[i] - binaural_smooth[i - 1]) > 2.0:
                # Verify this is a sustained change, not a spike
                if i < len(binaural_smooth) - 2:
                    # Check if change persists for at least 2 more samples
                    next_avg = np.mean(
                        binaural_smooth[i : min(i + 3, len(binaural_smooth))]
                    )
                    prev_avg = np.mean(binaural_smooth[max(0, i - 3) : i])

                    if abs(next_avg - prev_avg) > 1.5:
                        # This is a real section change
                        change_points.append(i)

            # Also check for significant carrier frequency changes
            elif abs(left_smooth[i] - left_smooth[i - 1]) > 20.0:
                if i < len(left_smooth) - 2:
                    next_avg = np.mean(left_smooth[i : min(i + 3, len(left_smooth))])
                    prev_avg = np.mean(left_smooth[max(0, i - 3) : i])

                    if abs(next_avg - prev_avg) > 15.0:
                        change_points.append(i)

        change_points.append(len(freq_timeline) - 1)  # Add end point

        # Remove duplicate change points that are too close together (within 30 seconds)
        filtered_change_points: List[int] = [change_points[0]]
        for cp in change_points[1:]:
            if times[cp] - times[filtered_change_points[-1]] > 30:
                filtered_change_points.append(cp)

        change_points = filtered_change_points

        # Create sections from change points
        sections: List[Dict[str, Any]] = []
        for i in range(len(change_points) - 1):
            start_idx = change_points[i]
            end_idx = change_points[i + 1]

            start_time = times[start_idx]
            end_time = times[end_idx] if end_idx < len(times) else self.duration

            # Calculate average frequencies for this section
            section_freqs = freq_timeline[start_idx : end_idx + 1]
            avg_binaural = float(np.mean([f["binaural_beat"] for f in section_freqs]))
            avg_left = float(np.mean([f["left_freq"] for f in section_freqs]))
            avg_amplitude = float(np.mean([f["amplitude"] for f in section_freqs]))

            # Determine section type based on characteristics
            section_num = i + 1
            total_sections = len(change_points) - 1

            if (
                section_num == 1
                and avg_amplitude < float(np.mean(amplitude_series)) * 0.7
            ):
                section_type = "intro"
            elif (
                section_num == total_sections
                and avg_amplitude < float(np.mean(amplitude_series)) * 0.7
            ):
                section_type = "outro"
            else:
                # Name based on binaural beat frequency
                if avg_binaural < 4:
                    wave_type = "delta"
                elif avg_binaural < 8:
                    wave_type = "theta"
                elif avg_binaural < 12:
                    wave_type = "alpha"
                elif avg_binaural < 30:
                    wave_type = "beta"
                else:
                    wave_type = "gamma"

                section_type = f"section_{section_num}_{wave_type}"

            sections.append(
                {
                    "section_type": section_type,
                    "start_time": float(start_time),
                    "end_time": float(end_time),
                    "duration": float(end_time - start_time),
                    "avg_binaural_beat": avg_binaural,
                    "avg_left_freq": avg_left,
                }
            )

        print(f"Detected {len(sections)} sections:")
        for sec in sections:
            print(
                f"  {sec['section_type']:20s}: {sec['start_time']:6.1f}s - {sec['end_time']:6.1f}s "
                f"({sec['duration']:6.1f}s) - Binaural: {sec.get('avg_binaural_beat', 0):.2f} Hz"
            )

        return sections

    def analyze_section(self, section: Dict[str, Any]) -> Dict[str, Any]:
        """
        Analyze frequencies in a specific section.

        Args:
            section: Section dictionary with start_time and end_time

        Returns:
            dict with frequency analysis results
        """
        start_sample = int(section["start_time"] * self.sample_rate)
        end_sample = int(section["end_time"] * self.sample_rate)

        # Split section into chunks for temporal analysis
        chunk_samples = int(self.chunk_duration * self.sample_rate)
        hop_samples = int((self.chunk_duration - self.overlap) * self.sample_rate)

        chunks: List[Dict[str, Any]] = []

        for chunk_start in range(start_sample, end_sample - chunk_samples, hop_samples):
            chunk_end = chunk_start + chunk_samples
            chunk_time = chunk_start / self.sample_rate

            # Analyze left and right channels
            left_analysis = self.analyze_frequency_spectrum(
                self.left_channel, chunk_start, chunk_end
            )
            right_analysis = self.analyze_frequency_spectrum(
                self.right_channel, chunk_start, chunk_end
            )

            # Calculate binaural beat
            left_freq = left_analysis["dominant_frequency"]
            right_freq = right_analysis["dominant_frequency"]
            binaural_beat = abs(left_freq - right_freq)

            # Classify brainwave type
            if binaural_beat < 4:
                wave_type = "Delta"
            elif binaural_beat < 8:
                wave_type = "Theta"
            elif binaural_beat < 12:
                wave_type = "Alpha"
            elif binaural_beat < 30:
                wave_type = "Beta"
            else:
                wave_type = "Gamma"

            chunks.append(
                {
                    "time": float(chunk_time),
                    "left_freq": float(left_freq),
                    "right_freq": float(right_freq),
                    "binaural_beat": float(binaural_beat),
                    "wave_type": wave_type,
                    "left_top_freqs": left_analysis["top_frequencies"],
                    "right_top_freqs": right_analysis["top_frequencies"],
                    "amplitude": float(
                        (
                            left_analysis["rms_amplitude"]
                            + right_analysis["rms_amplitude"]
                        )
                        / 2
                    ),
                }
            )

        # Calculate average for the section
        if chunks:
            avg_binaural = float(np.mean([c["binaural_beat"] for c in chunks]))
            avg_left = float(np.mean([c["left_freq"] for c in chunks]))
            avg_right = float(np.mean([c["right_freq"] for c in chunks]))
        else:
            avg_binaural = 0.0
            avg_left = 0.0
            avg_right = 0.0

        return {
            "section_type": section["section_type"],
            "start_time": section["start_time"],
            "end_time": section["end_time"],
            "chunks": chunks,
            "average": {
                "left_freq": avg_left,
                "right_freq": avg_right,
                "binaural_beat": avg_binaural,
            },
        }

    def analyze_full(self) -> Dict[str, Any]:
        """
        Perform complete analysis of the audio file.

        Returns:
            dict with complete analysis results
        """
        self.load_audio()
        sections = self.detect_sections()

        print("\nAnalyzing frequency content in each section...")

        results: Dict[str, Any] = {
            "file": str(self.audio_file),
            "duration": float(self.duration),
            "sample_rate": int(self.sample_rate),
            "sections": [],
        }

        for section in sections:
            print(f"\nAnalyzing {section['section_type']} section...")
            section_analysis = self.analyze_section(section)
            results["sections"].append(section_analysis)

            # Print summary
            avg = section_analysis["average"]
            print(f"  Average Left: {avg['left_freq']:.2f} Hz")
            print(f"  Average Right: {avg['right_freq']:.2f} Hz")
            print(f"  Average Binaural Beat: {avg['binaural_beat']:.2f} Hz")

        return results

    def print_report(self, results: Dict[str, Any]) -> None:
        """Print a human-readable report of the analysis."""
        print("\n" + "=" * 80)
        print(f"HEMI-SYNC FREQUENCY ANALYSIS REPORT")
        print("=" * 80)
        print(f"File: {results['file']}")
        print(
            f"Duration: {results['duration']:.1f} seconds ({results['duration'] / 60:.1f} minutes)"
        )
        print(f"Sample Rate: {results['sample_rate']} Hz")
        print("=" * 80)

        for section in results["sections"]:
            print(f"\n{section['section_type'].upper()} SECTION")
            print("-" * 80)
            print(
                f"Time Range: {section['start_time']:.1f}s - {section['end_time']:.1f}s"
            )
            print(f"Duration: {section['end_time'] - section['start_time']:.1f}s")
            print(f"\nAverage Frequencies:")
            print(f"  Left Channel:    {section['average']['left_freq']:8.2f} Hz")
            print(f"  Right Channel:   {section['average']['right_freq']:8.2f} Hz")
            print(f"  Binaural Beat:   {section['average']['binaural_beat']:8.2f} Hz")

            if section["chunks"]:
                print(f"\nTemporal Analysis ({len(section['chunks'])} chunks):")
                print(
                    f"  {'Time':>8s}  {'Left Hz':>8s}  {'Right Hz':>8s}  {'Beat Hz':>8s}  {'Type':>8s}"
                )
                print("  " + "-" * 52)

                # Show up to 10 chunks
                for chunk in section["chunks"][:10]:
                    print(
                        f"  {chunk['time']:8.1f}  {chunk['left_freq']:8.2f}  "
                        f"{chunk['right_freq']:8.2f}  {chunk['binaural_beat']:8.2f}  "
                        f"{chunk['wave_type']:>8s}"
                    )

                if len(section["chunks"]) > 10:
                    print(f"  ... ({len(section['chunks']) - 10} more chunks)")

        print("\n" + "=" * 80)


def main():
    parser = argparse.ArgumentParser(
        description="Analyze Hemi-Sync audio files to extract frequency information"
    )
    parser.add_argument(
        "audio_file", help="Path to audio file (FLAC, OGG, MP3, WAV, etc.)"
    )
    parser.add_argument("-o", "--output", help="Output JSON file for results")
    parser.add_argument(
        "-c",
        "--chunk-duration",
        type=int,
        default=30,
        help="Duration of analysis chunks in seconds (default: 30)",
    )
    parser.add_argument(
        "--overlap",
        type=int,
        default=15,
        help="Overlap between chunks in seconds (default: 15)",
    )

    args = parser.parse_args()

    # Check if file exists
    if not os.path.exists(args.audio_file):
        print(f"Error: File not found: {args.audio_file}")
        sys.exit(1)

    # Create analyzer
    analyzer = CoherenceAnalyzer(
        args.audio_file, chunk_duration=args.chunk_duration, overlap=args.overlap
    )

    # Run analysis
    try:
        results = analyzer.analyze_full()

        # Print report
        analyzer.print_report(results)

        # Save JSON if requested
        if args.output:
            with open(args.output, "w") as f:
                json.dump(results, f, indent=2)
            print(f"\nResults saved to: {args.output}")

    except Exception as e:
        print(f"\nError during analysis: {e}")
        import traceback

        traceback.print_exc()
        sys.exit(1)


if __name__ == "__main__":
    main()
