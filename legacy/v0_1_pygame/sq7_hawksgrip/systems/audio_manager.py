"""
Audio Manager

Manages pygame mixer initialization and procedural generation of the "scream" using numpy.
The scream is a high-pitched synthetic screech that ramps up in frequency.
"""
import numpy as np
import pygame
import config


class AudioManager:
    def __init__(self):
        pygame.mixer.init(frequency=config.AUDIO_SAMPLE_RATE, size=-16, channels=2)
        self.active_sounds = []
        self.scream_waveform = self._generate_scream()

    def _generate_scream(self) -> pygame.mixer.Sound:
        """
        Procedurally generates the terminal 'scream' sound.
        Uses a sine wave sweep from START_FREQ to END_FREQ.
        """
        sample_rate = config.AUDIO_SAMPLE_RATE
        duration = config.SCREAM_DURATION_SEC
        n_samples = int(sample_rate * duration)
        
        t = np.linspace(0, duration, n_samples, False)
        
        # Frequency sweep (Chirp)
        # Linear interpolation of frequency
        freqs = np.linspace(config.SCREAM_START_FREQ, config.SCREAM_END_FREQ, n_samples)
        
        # Calculate phase by integrating frequency over time
        phase = 2 * np.pi * np.cumsum(freqs) / sample_rate
        
        # Generate waveform
        wave = np.sin(phase)
        
        # Apply envelope (fade in/out) to avoid clicking
        envelope = np.ones_like(wave)
        fade_in_len = int(sample_rate * 0.1)
        fade_out_len = int(sample_rate * 0.2)
        
        envelope[:fade_in_len] = np.linspace(0, 1, fade_in_len)
        envelope[-fade_out_len:] = np.linspace(1, 0, fade_out_len)
        
        wave *= envelope
        
        # Normalize volume
        wave = wave * config.SCREAM_VOLUME
        
        # Convert to 16-bit integer range for pygame
        wave_int16 = (wave * 32767).astype(np.int16)
        
        # Stereo: duplicate mono channel
        stereo_wave = np.column_stack((wave_int16, wave_int16))
        
        return pygame.mixer.Sound(buffer=stereo_wave)

    def play_scream(self):
        """Plays the pre-generated scream sound."""
        self.scream_waveform.play()

    def cleanup(self):
        """Stops all active sounds."""
        pygame.mixer.stop()
