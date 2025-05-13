import librosa
from itertools import tee, islice, chain
import numpy as np

# Move away from tf since there's no model currently
import asyncio

async def get_mse_mat(waveform, sr):
	get_beat_track_r = get_beat_track(waveform, sr)

	def previous_and_next(some_iterable):
		prevs, items, nexts = tee(some_iterable, 3)
		prevs = chain([None], prevs)
		nexts = chain(islice(nexts, 1, None), [None])
		return zip(prevs, items, nexts)
	
	def mse(A,B):
		mean = (np.squeeze(A - B)**2).mean()
		# print(mean)
		return mean
	vmse = np.vectorize(mse, signature='(m),(m)->()')
	def matmse(A,B):
		return vmse(A,B).reshape(-1,1)
	vmatmse = np.vectorize(matmse, signature='(m),(n,m)->(n,1)')


	beat_track, tempo = await get_beat_track_r
	beats = beat_track.shape[0]
	beat_samples = np.array([np.mean(await get_spectrogram(waveform[p_sample:sample, :], pad=False), axis=0) for p_sample, sample, _ in previous_and_next(beat_track)])
	# print(beat_samples.shape)

	mse_arrays = [np.concatenate([np.zeros((i,1)),vmatmse(beat_samples[i,:,1],beat_samples[i:,:,1])]) for i in range(beats)]

	mse_mat = np.column_stack(mse_arrays)
	# print(mse_mat.shape)
	return mse_mat,beat_track, tempo