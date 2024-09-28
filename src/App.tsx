import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/tauri";
import React, { FC, useCallback, useEffect, useRef, useState } from "react";
import "./App.css";

import { Button } from "@/components/ui/button";
import { map } from "lodash-es";
import { HiOutlinePause, HiOutlinePlay } from "react-icons/hi2";
import { subscribeStateSync, useGlobalAppStore } from "./states/global-states";

listen("openFile", (e) => {
	console.log(e.payload);
	console.log("file selected");
});

export function animationInterval(
	ms: number,
	signal: AbortSignal,
	callback: (time: DOMHighResTimeStamp) => void,
) {
	// Prefer currentTime, as it'll better sync animtions queued in the
	// same frame, but if it isn't supported, performance.now() is fine.
	const start = Number(document.timeline?.currentTime || performance.now());

	function frame(time: DOMHighResTimeStamp) {
		if (signal.aborted) return;
		callback(time);
		scheduleFrame(time);
	}

	function scheduleFrame(time: DOMHighResTimeStamp) {
		const elapsed = time - start;
		const roundedElapsed = Math.round(elapsed / ms) * ms;
		const targetNext = start + roundedElapsed + ms;
		const delay = targetNext - performance.now();
		setTimeout(() => requestAnimationFrame(frame), delay);
	}

	scheduleFrame(start);
}

function getTextWidth(text: string, font?: any) {
	const canvas = document.createElement("canvas");
	const context = canvas.getContext("2d");

	if (context) {
		context.font = font || getComputedStyle(document.body).font;

		return context.measureText(text).width;
	}

	return 0;
}

const SecondsMS = 1 * 1000;
const MinutesMS = 60 * SecondsMS;
const HoursMS = 60 * MinutesMS;

const SECOND_SCALE = 10;

const Duration: FC<{}> = () => {
	const isPaused = useGlobalAppStore((store) => store.playback?.isPaused);
	const playback = useGlobalAppStore((store) => store.playback);
	const [controller, setController] = useState(new AbortController());
	let [playbackTime, setPlaybackTime] = useState(0);

	const totalFrames = useGlobalAppStore(
		(store) => store.playback?.totalFrames,
	);

	useEffect(() => {
		const playTime =
			((playback?.totalFrames ?? 0) / (playback?.sampleRate ?? 0)) * 1000;
		Number.isNaN(playTime) || setPlaybackTime(playTime);
	}, [totalFrames]);

	useEffect(() => {
		if (isPaused == false) {
			const controller = new AbortController();
			setController(controller);
			const playTime =
				((playback?.totalFrames ?? 0) / (playback?.sampleRate ?? 0)) *
				1000;
			Number.isNaN(playTime) || setPlaybackTime(playTime);

			animationInterval(
				50,
				controller.signal,
				(_: DOMHighResTimeStamp) => {
					setPlaybackTime((time) => time + 50);
				},
			);
		} else if (isPaused == true) {
			controller.abort();
			const pauseTime =
				((playback?.totalFrames ?? 0) / (playback?.sampleRate ?? 0)) *
				1000;

			Number.isNaN(pauseTime) || setPlaybackTime(pauseTime);
		}
		// return () => {
		// 	controller.abort();
		// }
	}, [isPaused]);

	const h = ~~(playbackTime / HoursMS);
	playbackTime -= h * HoursMS;
	const m = ~~(playbackTime / MinutesMS);
	playbackTime -= m * MinutesMS;
	const s = ~~(playbackTime / SecondsMS);
	playbackTime -= s * SecondsMS;
	const ms = ~~playbackTime;

	const formattedH = h.toString().padStart(2, "0");
	const formattedM = m.toString().padStart(2, "0");
	const formattedS = s.toString().padStart(2, "0");
	const formattedMS = ms.toString().padStart(3, "0");

	return (
		<div>
			{formattedH}:{formattedM}:{formattedS}.{formattedMS}
		</div>
	);
};

const TickBar: FC<{}> = () => {
	const ref = useRef<HTMLDivElement | null>(null);

	const textSize = getTextWidth("00:00:00");
	const timeSize = textSize + 10 * 2;

	const interval = Math.ceil(timeSize / SECOND_SCALE);
	const sampleRate = useGlobalAppStore((store) => store.playback?.sampleRate);

	const [ticks, setTicks] = useState<number[]>([]);
	const [width, setwidth] = useState(0);

	useEffect(() => {
		const observer = new ResizeObserver((entries) => {
			setwidth(entries[0].contentRect.width);
		});
		ref.current && observer.observe(ref.current);

		return () => (ref.current ? observer.unobserve(ref.current) : void 0);
	}, []);

	useEffect(() => {
		const localTicks = [];
		for (let i = 0; i < (width ?? 0) / SECOND_SCALE; i += interval) {
			localTicks.push(i);
		}
		setTicks(localTicks);
	}, [width]);

	const onClick = useCallback(
		async (event: React.MouseEvent<HTMLDivElement, MouseEvent>) => {
			// event.

			const rect = ref.current?.getBoundingClientRect();
			var x = event.clientX - (rect?.left ?? 0); //x position within the element.

			const time = x / SECOND_SCALE;
			const sample = time * (sampleRate ?? 0);
			console.log({ time, sample });
			await invoke("try_seek", { pos: time });
		},
		[ref, sampleRate],
	);

	return (
		<div
			ref={ref}
			onClick={(...args) => onClick(...args)}
			className="relative table-row"
		>
			{map(ticks, (tick) => {
				let seconds = tick;
				let minutes = seconds / 60;
				// let hours = minutes / 60;
				minutes %= 60;
				seconds %= 60;
				// const formattedH = (~~hours).toString().padStart(2, "0");
				const formattedM = (~~minutes).toString().padStart(2, "0");
				const formattedS = (~~seconds).toString().padStart(2, "0");
				return (
					<>
						<div
							className="absolute h-1/2 w-[1.5px] bg-black"
							style={{ left: tick * SECOND_SCALE }}
						/>
						<span
							className="absolute bottom-0"
							style={{ left: tick * SECOND_SCALE }}
						>
							{formattedM}:{formattedS}
						</span>
					</>
				);
			})}
		</div>
	);
};

const Playhead: FC<{}> = () => {
	const ref = useRef<HTMLDivElement | null>(null);

	const [controller, setController] = useState(new AbortController());
	const [playbackHeadPosition, setPlaybackHeadPosition] = useState(0);
	const [lastHeartbeat, setLastHeartbeat] = useState(0);

	const isPaused = useGlobalAppStore((store) => store.playback?.isPaused);
	const totalFrames = useGlobalAppStore(
		(store) => store.playback?.totalFrames,
	);
	const sampleRate = useGlobalAppStore((store) => store.playback?.sampleRate);

	// For this, when you seek, the time between the seek and the second heart beat is lost
	useEffect(() => {
		let updatedTotalFrames = (totalFrames ?? 0) / (sampleRate ?? 0);
		updatedTotalFrames = Number.isNaN(updatedTotalFrames)
			? 0
			: updatedTotalFrames;

		ref.current && ref.current.classList.add("transition-none");

		const seekTime = performance.now();
		setPlaybackHeadPosition(updatedTotalFrames * SECOND_SCALE);
		if (isPaused == false) {
			setTimeout(() => {
				const diff = lastHeartbeat - seekTime;
				if (Math.abs(diff) <= 1000) {
					if (diff <= 0) {
						setPlaybackHeadPosition(
							(pos) => pos + (diff / 1000 + 1) * SECOND_SCALE,
						);
					} else {
						setPlaybackHeadPosition(
							(pos) => pos + (diff / 1000) * SECOND_SCALE,
						);
					}
				}
				ref.current && ref.current.classList.remove("transition-none");
			}, 20);
		}
	}, [totalFrames, isPaused]);

	useEffect(() => {
		if (isPaused == false) {
			const controller = new AbortController();
			setController(controller);

			setPlaybackHeadPosition((pos) => pos + SECOND_SCALE);
			ref.current && ref.current.classList.remove("transition-none");

			animationInterval(
				1000,
				controller.signal,
				(_: DOMHighResTimeStamp) => {
					setLastHeartbeat(performance.now());
					setPlaybackHeadPosition((pos) => pos + SECOND_SCALE);
				},
			);
		} else if (isPaused == true) {
			controller.abort();
			let pauseTime = (totalFrames ?? 0) / (sampleRate ?? 0);

			ref.current && ref.current.classList.add("transition-none");

			pauseTime = Number.isNaN(pauseTime) ? 0 : pauseTime;

			setPlaybackHeadPosition(pauseTime * SECOND_SCALE);
		}
		// return () => {
		// 	controller.abort();
		// }
	}, [isPaused]);

	return (
		<div
			className="absolute z-50 h-full w-[1px] bg-red-600 transition-all duration-1000 ease-linear"
			style={{ left: playbackHeadPosition }}
			ref={ref}
		/>
	);
};
function App() {
	async function togglePlayback() {
		// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
		await invoke("toggle_playback");
	}
	// async function getBeats() {
	// 	// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
	// 	await invoke("get_beats");
	// }

	async function addTrack() {
		await invoke("add_track");
	}

	const tracks = useGlobalAppStore((store) => store.tracks);
	const isPaused = useGlobalAppStore(
		(store) => store.playback?.isPaused ?? true,
	);
	// const playback = useGlobalAppStore((store) => store.playback);

	const onKeyDown = useCallback((event: KeyboardEvent) => {
		if (event.code == "Space" && event.target == document.body) {
			event.preventDefault();
			togglePlayback();
		}
		// if (event.code == "Space") {
		// }
	}, []);

	useEffect(() => {
		window.addEventListener("keydown", onKeyDown);
		return () => window.removeEventListener("keydown", onKeyDown);
	}, []);

	// useSpring({
	// 	to: async (next) => {
	// 		while (true) {
	// 			await next({ x: 1 });
	// 			await next({ x: 10 });
	// 		}
	// 	},
	// });

	useEffect(() => {
		console.log(tracks);
	}, [tracks]);

	useEffect(() => {
		const unsubscribeStateSync = subscribeStateSync();
		return () => {
			unsubscribeStateSync.then((unsubscribe) => unsubscribe());
		};
	}, []);

	return (
		<div className="grid h-[100vh] grid-rows-[auto_1fr]">
			<div className="flex p-2">
				<Button onClick={togglePlayback}>
					{isPaused ? (
						<HiOutlinePlay className="h-6 w-6" />
					) : (
						<HiOutlinePause className="h-6 w-6" />
					)}
				</Button>
			</div>
			<div className="grid grid-rows-[1fr_20rem] overflow-hidden p-2">
				<div className="grid grid-cols-[10rem_1fr] overflow-y-auto overflow-x-hidden">
					<div className="grid grid-rows-[2rem_1fr]">
						<Duration />
						<div className="space-y-1">
							{map(tracks, (track) => {
								return <div className="h-16">{track.name}</div>;
							})}
							<Button onClick={addTrack}>Add Track</Button>
						</div>
					</div>
					<div className="overflow-x-auto">
						<div className="relative grid h-full grid-rows-[2rem_repeat(auto-fit,_minmax(4rem,_4rem))] gap-y-1 overflow-y-hidden">
							<TickBar />
							<Playhead />
							{map(tracks, (track) => {
								return (
									<div className="flex pr-[50vw]">
										{map(track.clips, (clip) => {
											const width =
												(clip.audio.length /
													clip.audio.sampleRate) *
												SECOND_SCALE;
											return (
												<div
													className="inline-block h-full min-w-0 shrink-0 bg-gray-500 p-1"
													style={{ width }}
												>
													<span className="sticky left-1">
														{clip.name}
													</span>
												</div>
											);
										})}
									</div>
								);
							})}
						</div>
					</div>
				</div>
				<div></div>
			</div>
		</div>
	);
}

export default App;
