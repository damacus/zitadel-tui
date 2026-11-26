---
name: tui-demo
description: Record real terminal interfaces with fictional data and produce animated GIFs for GitHub READMEs, release notes, and pull requests. Use for TUI walkthroughs, asciinema casts, and repeatable documentation recordings.
---

# Record a TUI demonstration

1. Run the real binary against its offline demo mode or a local fixture. Use
   fictional records and keep credentials, live hostnames and personal details
   out of the capture.
2. Choose a short workflow with a visible result. Show the starting screen, the
   relevant navigation and the result. Start near 100 columns by 26 rows.
3. Record manually with `asciinema rec` or use [record.py](scripts/record.py)
   for repeatable keystrokes on macOS or Linux. The script writes an asciicast
   v2 file from a JSON array. Each step has `keys`, `hold` in seconds and
   optional `expect` text from the output. Terminal cursor sequences can split
   words, so check a distinctive fragment.
4. Render with `agg` and an idle limit at least as long as the longest pause.
   For scripted casts, pass the recorded final pause explicitly. The renderer
   coalesces identical frames.
5. Inspect the opening, key intermediate screens and ending. Check labels,
   borders, the visible result and the loop.
6. Put the GIF in the repository’s docs assets and embed it in the README with
   a relative path, descriptive alt text and the demo command. Verify the
   GitHub README after publishing.

## Record a scripted walkthrough

This JSON presses Enter and waits 1.5 seconds.

```json
[{"keys":"\r","hold":1.5,"expect":"Details"}]
```

```sh
python3 scripts/record.py --ready "Demo mode" --steps walkthrough.json \
  --output demo.cast -- ./target/debug/example --demo
```

Use the [Zitadel walkthrough](references/zitadel-walkthrough.json) with
`zitadel-tui --demo` and `--ready Northwind` to reproduce the README example.

## Render a scripted cast

Use the terminal reset timestamp and the preceding output timestamp to
calculate the final frame delay.

```sh
hold=$(python3 -c '
import json, sys
with open(sys.argv[1]) as cast:
    next(cast)
    events = [json.loads(line) for line in cast]
print(max(0.1, events[-1][0] - events[-2][0]))
' demo.cast)
agg --theme github-dark --font-size 15 --idle-time-limit 10 --fps-cap 12 \
  --last-frame-duration "$hold" demo.cast demo.gif
```
