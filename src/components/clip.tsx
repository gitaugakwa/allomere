import { SECOND_SCALE } from "@/App";
import useGlobalAppStore from "@/states/global-states";
import { invoke } from "@tauri-apps/api/core";
import { times } from "lodash-es";
import { FC, useCallback, useMemo } from "react";


export const Clip: FC<{clip: any}> = ({clip}) => { 
	const width = (clip.audio.length / clip.audio.sampleRate) * SECOND_SCALE;

	const loopWidth = useMemo(() => {
		if (clip.audio.loopStart) {
			const loopFrameLength = clip.audio.loopEndFrame - clip.audio.loopStartFrame

			return (loopFrameLength / clip.audio.sampleRate) * SECOND_SCALE
		}
		return null
	}, [clip.audio.loopStart, clip.audio.loopStartFrame, clip.audio.loopEndFrame])
	
	// const currentClipFocus = useGlobalAppStore(state=> state.currentClipFocus)
	const setCurrentClipFocus = useGlobalAppStore(state=> state.setCurrentClipFocus)
	const setCurrentClipFocusAudioData = useGlobalAppStore(state => state.setCurrentClipFocusAudioData)
	
	const onClipClick = useCallback(() => {
		console.log("Clicked", clip.id)
		console.log(clip)
		// setCurrentClipFocus(clip.id)
		invoke<{id:number, path:string}>("get_clip", { id: clip.id }).then((clip) => {
				console.log(clip)
			setCurrentClipFocus(clip)
			invoke("get_audio_data", { path: clip.path }).then((data) => {
				console.log(data)
				setCurrentClipFocusAudioData(data)
			})
			})
	}, [setCurrentClipFocus])
	
	
	return (
		<div
			className='grid [grid-template-areas:_"stack"] h-full min-w-0 shrink-0 bg-gray-500'
			style={{ width: loopWidth ? undefined : width }}
			onClick={onClipClick}
		>
			<div className="flex h-full [grid-area:_stack]">

				{loopWidth && <>
					<div style={{ width: (clip.audio.loopStartFrame / clip.audio.sampleRate) * SECOND_SCALE }} className="h-full border-black border border-r-0 bg-gray-500" />
					<span className="flex">
						{times(10, () => <div style={{ width: loopWidth }} className="h-full border border-black border-r-0 last:border-r first:bg-gray-500 bg-green-500"/>)}
					</span>
					<div style={{ width: width - loopWidth - (clip.audio.loopStartFrame / clip.audio.sampleRate) * SECOND_SCALE }} className="h-full border-black border border-l-0 bg-gray-500" />
				</>}
				
			</div>
			{/* <div style={{width: loopWidth}} className="h-2 bg-black"></div> */}
			<div className="[grid-area:_stack]">
				<span className="sticky left-1 top-1">
					{clip.name}
				</span>
			</div>
		</div>
	);
}