import { Playhead, SECOND_SCALE, TickBar } from "@/App"
import { cn } from "@/lib/utils"
import useGlobalAppStore from "@/states/global-states"
import { invoke } from "@tauri-apps/api/core"
import { map } from "lodash-es"
import { useCallback, useEffect, useRef, useState, type FC } from "react"
import Xarrow from 'react-xarrows';
import { DropdownMenu, DropdownMenuContent, DropdownMenuTrigger } from "./ui/dropdown-menu"
import { DropdownMenuItem } from "@radix-ui/react-dropdown-menu"
import { Button } from "./ui/button"




export const ClipFocusSection: FC<{}> = ({ }) => {

	const clip = useGlobalAppStore(state => state.currentClipFocus)
	const currentClipFocusAudioData = useGlobalAppStore(state => state.currentClipFocusAudioData)

	const startRef = useRef<HTMLDivElement|null>(null)
	const endRef = useRef<HTMLDivElement | null>(null)
	
	const [preferredBeatTransition, setPreferredBeatTransition] = useState<Map<number,[string, number][]>>(new Map())


	const [loopStart, setLoopStart] = useState(clip.audio.loopStart && clip.audio.loopStartFrame)
	const [loopEnd, setLoopEnd] = useState(clip.audio.loopStart && clip.audio.loopEndFrame)

	const scale = 5

	const setLoop = useCallback((startFrame: number, endFrame:number) => {
		invoke("set_clip_loop_frames", {id: clip.id, startFrame, endFrame }).then(()=> {
			console.log("Loop created")
		})
	}, [clip])
	const clearLoop = useCallback(() => {
		invoke("clear_clip_loop", {id: clip.id }).then(()=> {
			console.log("Loop Cleared")
		})
	}, [clip])

	useEffect(() => {
		if (loopStart) {
			const loopStartIndex = (currentClipFocusAudioData.beatTrack as []).findIndex((_) => _ == loopStart)
			invoke<Record<number,number>>("get_clip_preferred_transition_beats", { id: clip.id, beat: loopStartIndex, count: 5 }).then((preferredTransitionBeats) => {
				let sortedEntries = (Object.entries(preferredTransitionBeats).sort(([, a], [, b]) => a - b));
				// sortedEntries = sortedEntries.filter(([_])=> _ < currentClipFocusAudioData.beatTrack.length)
				// preferredTransitionBeats = Object.fromEntries(Object.entries(preferredTransitionBeats).sort(([, a], [, b]) => a - b))
				console.log(sortedEntries)
				setPreferredBeatTransition((_) => {
					_.set(loopStart, sortedEntries)
					return new Map(_)
				})
			} )
		}
		if (loopEnd) {
			const loopEndIndex = (currentClipFocusAudioData.beatTrack as []).findIndex((_)=> _==loopEnd)
			invoke<Record<number,number>>("get_clip_preferred_transition_beats", { id: clip.id, beat: loopEndIndex, count: 5 }).then((preferredTransitionBeats) => {
				let sortedEntries = (Object.entries(preferredTransitionBeats).sort(([, a], [, b]) => a - b));
				// sortedEntries = sortedEntries.filter(([_])=> _ < currentClipFocusAudioData.beatTrack.length)
				// sortedEntries.map(_=> [_[0].toString(), _[1]])
				// preferredTransitionBeats = Object.fromEntries(sortedEntries.map(_=> [_[0].toString(), _[1]]))
				console.log(sortedEntries)
				setPreferredBeatTransition((_) => {
					_.set(loopEnd, sortedEntries)
					return new Map(_)
				})
			} )
		}
	}, [loopEnd, loopStart])

	// console.log(preferredBeatTransition)
	// console.log(loopStart)

	
	// const [focusedClip, setFocusedClip] = useState<null | unknown>(null)
	
	// useEffect(() => {
	// 	if (currentClipFocus) {
	// 		invoke("get_clip", { id: currentClipFocus }).then((clip) => {
	// 			console.log(clip)
	// 			setFocusedClip(clip)
	// 		})
	// 	}
	// }, [currentClipFocus])

	if (!clip) {
		return <div />
	}

	const width = (clip.audio.length / clip.audio.sampleRate) * SECOND_SCALE * scale;

	return <div className={cn("grid gap-x-4 overflow-hidden", { "grid-cols-[1fr_20rem]": loopStart || loopEnd })}>
		<div className="grid grid-rows-[2rem_1fr]">
			<h1>{clip?.name}</h1>
			<div className="overflow-x-auto grow w-full">
							<div className="relative grid h-full grid-rows-[2rem_1fr] gap-y-1 overflow-y-hidden">
			
				<TickBar scale={scale} />
				{/* <div className="block w-full grow max-w-none" /> */}
				<Playhead scale={scale} />
					<div
						style={{ width }}
						className="relative h-10 mr-[50vw]">
						{map(currentClipFocusAudioData?.beatTrack, (beat) => {
							return <DropdownMenu >
								<DropdownMenuTrigger asChild>
			
						<div className={cn("absolute h-1/2 w-[0px] border border-black bg-black cursor-pointer", { "border-red-600": beat == clip.audio.loopEndFrame,"border-green-600": beat == clip.audio.loopStartFrame})}
									style={{ left: (beat / clip.audio.sampleRate) * SECOND_SCALE * scale }} ref={beat == clip.audio.loopEndFrame ? endRef : beat == clip.audio.loopStartFrame? startRef: undefined} />
								</DropdownMenuTrigger>
								<DropdownMenuContent>
									<DropdownMenuItem onClick={()=>{setLoopStart(beat)}} >Start Loop</DropdownMenuItem>
									<DropdownMenuItem onClick={()=>{setLoopEnd(beat)}} >End Loop</DropdownMenuItem>
								</DropdownMenuContent>
									</DropdownMenu>
					})}
				</div>
					<Xarrow start={endRef} end={startRef} curveness={0.2} />
				</div>
			</div>
		</div>
		{(loopStart || loopEnd || null) && (<div className="flex flex-col">
			<h2>Create new loop</h2>
			<div className="flex justify-between">
				<span>Start</span>
				<span>End</span>
			</div>
			<div className="grid grid-cols-3 font-bold">
				<span>{(loopStart || null) && currentClipFocusAudioData?.beatTrack.findIndex((_:number)=>_==loopStart)}</span>
				<span className="justify-self-center">-&gt;</span>
				<span className="text-end">{(loopEnd || null) && currentClipFocusAudioData?.beatTrack.findIndex((_: number)=>_==loopEnd)}</span>
			</div>
			<div className="grid grid-cols-2">
				<div>{(preferredBeatTransition.get(loopStart) || []).map((_) => <div className="cursor-pointer" onClick={()=> setLoopEnd(currentClipFocusAudioData?.beatTrack[_[0]])}>{_[0]}</div>)}</div>
				<div className="text-end">{(preferredBeatTransition.get(loopEnd) || []).map((_) => <div className="cursor-pointer" onClick={()=> setLoopStart(currentClipFocusAudioData?.beatTrack[_[0]])}>{_[0]}</div>)}</div>
			</div>
			<div className="mt-auto">{((loopStart && loopEnd) || null) && <Button className="w-full" onClick={() => {
				setLoop(loopStart, loopEnd)
			}}>Create Loop</Button>}</div>
			<div className="mt-1">{((clip.audio.loopStart) || null) && <Button className="w-full" onClick={() => {
				clearLoop()
			}}>Clear Loop</Button>}</div>
			{/* {loopStart && <div>
				{JSON.stringify(preferredBeatTransition.get(loopStart))}
			</div>}
			{loopEnd && <div>
				{JSON.stringify(preferredBeatTransition.get(loopEnd))}
			</div>} */}
		</div>) }
		</div>
}
