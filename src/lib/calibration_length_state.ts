import { PersistedState } from "runed";

// Distance walked during a single calibration step.
// It says nothing about the distance covered in an experiment trial.
function get_custom_state(): PersistedState<number | ""> {
	const CalibrationLengthState: PersistedState<number | ""> = new PersistedState("calibration_length", 10);
	const originalDescriptor = Object.getOwnPropertyDescriptor(
		Object.getPrototypeOf(CalibrationLengthState),
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
			CalibrationLengthState,
			'current',
			{
				set: newSetter,
				get: originalDescriptor.get,
				configurable: true,
				enumerable: originalDescriptor.enumerable
			}
		);
	}
	return CalibrationLengthState;
}

export const CalibrationLengthState = get_custom_state();
