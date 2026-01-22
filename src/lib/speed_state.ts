import { PersistedState } from "runed";

export const Nothing = "";

function get_custom_state(): PersistedState<number | ""> {
	const SpeedState: PersistedState<number | ""> = new PersistedState("speed", 4.0);
	const originalDescriptor = Object.getOwnPropertyDescriptor(
		Object.getPrototypeOf(SpeedState),
		'current'
	);

	if (!originalDescriptor || !originalDescriptor.set) {
		console.error("Could not find original setter for 'current'. Patching aborted.");
	} else {
		const originalSetter = originalDescriptor.set;

		const newSetter = function(this: { set: (newValue: number | null | "") => void; get: (() => any) | undefined; configurable: true; enumerable: boolean | undefined; }, newValue: number | null | "") {
			let valueToSet = newValue;
			if (newValue === null) {
				valueToSet = "";
			}

			originalSetter.call(this, valueToSet);
		};

		Object.defineProperty(
			SpeedState,
			'current',
			{
				set: newSetter,
				get: originalDescriptor.get,
				configurable: true,
				enumerable: originalDescriptor.enumerable
			}
		);
	}
	return SpeedState;
}


export const SpeedState = get_custom_state();
