export interface PlaybackState {
	isPaused: boolean;
	totalFrames: number;
	sampleRate: number;
}

export interface GlobalAppState {
	tracks: Array<{
		name: string;
		clips: Array<{
			path: string;
			name: string;
			audio: { length: number; sampleRate: number };
		}>;
	}>;
	check: boolean;

	playback?: PlaybackState;

	setTracks: (tracks: Array<{ name: string }>) => void;
	setCheck: (check: boolean) => void;
}
