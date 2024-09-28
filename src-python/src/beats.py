import librosa

def get_beats(path: str, sample_rate: int) -> tuple:

    y, sr = librosa.load(path, sr=sample_rate)

    print("sr: ", sr)
    tempo, beats = librosa.beat.beat_track(
        y=librosa.to_mono(y),
        sr=sr,
        tightness=1,
        hop_length=256,
        trim=False,
        units="samples"
        )

    return tempo, beats
