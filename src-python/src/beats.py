import librosa

def get_beats(path: str, sample_rate: int) -> tuple:

    y, sr = librosa.load(path, sr=sample_rate)

    print("path: ", path)
    print("sr: ", sr)
    tempo, beats = librosa.beat.beat_track(
        y=librosa.to_mono(y),
        sr=sr,
        # tightness=20,
        # hop_length=256,
        # trim=False,
        units="samples"
        )

    print("beats: ", len(beats))
    print("tempo: ", tempo)

    return tempo, beats
