from datasets import load_dataset
from transformers import ClapModel, ClapProcessor, AutoProcessor
import librosa
import numpy as np

import sounddevice as sd

import time  # Import the time module

model = ClapModel.from_pretrained("laion/larger_clap_music")
processor = ClapProcessor.from_pretrained("laion/larger_clap_music")

from typing import List

def get_audio_features(audio: List[np.ndarray], sample_rate: int):
    start_time = time.time()  # Record the start time
    # audio.

    # dataset = load_dataset("ashraq/esc50")
    # audio = [dataset["train"]["audio"][-1]["array"]]
    # sample_rate = dataset["train"]["audio"][-1]["sampling_rate"]
    # print(audio.size)
    # print(dataset["train"]["audio"][-1])

    # print(sample_rate)
    # sd.play(audio, 44100)

    inputs = processor(
        audios=[librosa.resample(y=audio, orig_sr=sample_rate, target_sr=48000) for audio in audio],
        return_tensors="pt",
        sampling_rate=48000
        )

    print(f"resampling took {time.time() - start_time:.2f} seconds")  # Log the duration
    audio_embed = model.get_audio_features(**inputs)

    print(f"features took {time.time() - start_time:.2f} seconds")  # Log the duration
    print(audio_embed.detach().numpy().shape)
    audio_embed = audio_embed.detach().numpy()
    audio_embed = [arr.tolist() for arr in audio_embed]

    print(f"get_audio_features took {time.time() - start_time:.2f} seconds")  # Log the duration

    # print(audio_embed.__len__())
    # print(audio_embed)
    return audio_embed

if __name__ == '__main__':
    dataset = load_dataset("ashraq/esc50")
    audio = dataset["train"]["audio"][-1]["array"]
    sample_rate = dataset["train"]["audio"][-1]["sampling_rate"]
    print((get_audio_features(audio=[audio], sample_rate=sample_rate)[0]))