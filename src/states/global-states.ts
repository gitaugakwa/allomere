import { GlobalAppState } from "@app/types/state";
import { emit as emitStateEvent, listen } from "@tauri-apps/api/event";
import { create } from "zustand";

import { EVENTS, GLOBAL_APP_STATE_MACRO } from "@app/constants";
import { set } from "lodash-es";
import { subscribeWithSelector } from "zustand/middleware";

const emit = (key: number, value: any) => {
	console.log(Object.keys(value)[0]);
	emitStateEvent(EVENTS.STATE_CHANGE_EVENT, {
		key: key,
		value: value,
	});
};

export const useGlobalAppStore = create(
	subscribeWithSelector<GlobalAppState>(() => ({
		tracks: [],
		check: false,
		setTracks: (tracks: Array<{ name: string }>) => {
			emit(GLOBAL_APP_STATE_MACRO.TRACKS, tracks);
		},
		setCheck: (check: boolean) => {
			emit(GLOBAL_APP_STATE_MACRO.CHECK, check);
		},
	})),
);

// const getKey = (key: number) => {
// 	if (key == GLOBAL_APP_STATE_MACRO.TRACKS) {
// 		return "tracks";
// 	} else if (key == GLOBAL_APP_STATE_MACRO.CHECK) {
// 		return "check";
// 	}
// };

const subscribeStateSync = async () => {
	const unsubscribeStateSyncListener = await listen(
		EVENTS.STATE_SYNC_EVENT,
		(event) => {
			const key = (event.payload as any).key;
			const value = JSON.parse((event.payload as any).value);
			console.log(event);

			useGlobalAppStore.setState((state) => ({
				...set(state, key, value),
			}));
		},
	);

	return async () => {
		unsubscribeStateSyncListener();
	};
};

export default useGlobalAppStore;
export { subscribeStateSync };
