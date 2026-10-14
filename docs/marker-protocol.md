# LsL marker protocol

The app publishes one marker per experiment event on the LsL stream

| | |
|---|---|
| name | `App Events` |
| type | `Markers` |
| channels | 1, `string`, irregular rate |

Each sample is a single JSON object. The `type` field says which type of event the marker describes
and therefore which fields the object has; inside a block, `state` says which moment of it. A
field that does not apply to a marker is absent.

## Markers

### `session`

The start of a session, published when the instructions open. It belongs to no block and no trial.

```json
{"type":"session"}
```

### `calibration`

One step of the speed calibration. A calibration has no block, `step` is its own 1 based counter.
`state` is `start`, `stop`, `discarded` or `confirmed` for the course of a step, and `result` for
the marker that closes the calibration, which is the only one carrying data.

```json
{"type":"calibration","step":2,"state":"start"}
{"type":"calibration","step":4,"state":"result","speed_kmh":4.23}
```

`speed_kmh` is the calibrated walking speed in km/h, rounded to two decimals.

### `stimulus`

A trial that shows an emotional image. `block` and `trial` are 1 based, `trial` counting inside
its block. `speed` is the walking condition of the trial, one of `none`, `very_slow`, `slow`,
`normal`, `fast`, `very_fast`.

`state` runs `baseline` → `stimulus` → `go` → `rating_prompt`, followed by `rating_valence` and
`rating_arousal` as the subject answers. Those two carry `rating`, the answer on the scale; which
of them appear depends on the rating settings of the run.

```json
{"type":"stimulus","block":1,"trial":3,"speed":"very_fast",
 "image_id":{"valence":"Low","arousal":"High","index":5},"state":"go"}
{"type":"stimulus","block":1,"trial":3,"speed":"very_fast",
 "image_id":{"valence":"Low","arousal":"High","index":5},
 "state":"rating_valence","rating":7}
```

`image_id` identifies the image: the quadrant it was drawn from (`valence` and `arousal`, each
`Low` or `High`) and its `index` inside that quadrant. The index is the position of the file in
the quadrant folder sorted by name, counting only `.webp` files, so it moves when the image set
changes; the trial CSV carries the file name next to it for that reason. `image_id` is `null` if
the image of the trial failed to load.

### `neutral`

A trial that shows a fixation cross in the image's place. Same fields as a stimulus trial minus
the image, and it is never rated: it stops at `rating_prompt`, where the subject only confirms.

```json
{"type":"neutral","block":2,"trial":5,"speed":"slow","state":"go"}
```

### `pause`

A break between blocks. It has no phases, no walking and nothing to rate, only the 1 based index
of the block it stands in for.

```json
{"type":"pause","block":3}
```

### `test`

A practice run opened from the instructions. It runs an ordinary stimulus plan and its markers
carry the same fields as `stimulus`; the tag is what keeps a recording from taking it for real
data.

```json
{"type":"test","block":1,"trial":2,"speed":"none",
 "image_id":{"valence":"High","arousal":"High","index":12},"state":"baseline"}
```

## Reading the stream

```python
from pylsl import StreamInlet, resolve_byprop
import json

inlet = StreamInlet(resolve_byprop("name", "App Events")[0])
while True:
    sample, timestamp = inlet.pull_sample()
    marker = json.loads(sample[0])
    print(timestamp, marker["type"], marker.get("state"))
```

The producing types live in `src-tauri/src/lsl.rs` (`LsLMarker` and the `state` enums next to it)
and are mirrored for the UI in `src/lib/lsl.ts`.
