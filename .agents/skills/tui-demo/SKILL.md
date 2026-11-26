---
name: tui-demo
description: Record real terminal interfaces with fictional data and produce animated GIFs for GitHub READMEs, release notes, and pull requests. Use for TUI walkthroughs, asciinema casts, and repeatable documentation recordings.
---

# Record a TUI demonstration

1. Run the real binary against its offline demo mode or a local fixture. Use fictional records and keep credentials, live hostnames and personal details out of the capture.
2. Choose a short workflow with a visible result. Show the starting screen, the relevant navigation and the result. Start near 100 columns by 26 rows and adjust to fit the interface.
3. Record manually using `asciinema rec` or use [record.py](scripts/record.py) for repeatable keystrokes on macOS or Linux. The script writes an asciicast v2 file. Give it a JSON array of steps with `keys`, `hold` in seconds, and optional `expect` text from the output. For example, `[{"keys":"\\r","hold":1.5,"expect":"Details"}]` presses Enter and waits 1.5 seconds. A terminal may split words with cursor sequences, so check a distinctive fragment.
4. Render with `agg --theme github-dark --font-size 15 --idle-time-limit 2 --fps-cap 12 demo.cast demo.gif`. Leave enough time for readers to follow each screen. Optimise the GIF if its size is disproportionate to the sequence.
5. Inspect the first frame, intermediate screens and final frame. Confirm that labels and borders are legible, the result appears, and the loop starts cleanly.
6. Put the GIF in the repository’s docs assets and embed it in the README with a relative link and descriptive alt text. Include the command readers can run. Verify the GitHub README after publishing.

Example recorder invocation

```sh
python3 scripts/record.py --ready "Demo mode" --steps walkthrough.json --output demo.cast -- ./target/debug/example --demo
```

Use the [Zitadel walkthrough](references/zitadel-walkthrough.json) with `zitadel-tui --demo` and `--ready Northwind` to reproduce the README example.
