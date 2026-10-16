# fine-volume

Make the volume keys (or a keyboard volume knob) on macOS always move in
quarter steps — 64 levels instead of 16 — with the native volume overlay.

macOS already does this when you hold <kbd>Option</kbd>+<kbd>Shift</kbd> while
changing the volume. fine-volume holds them for you: it catches each volume
up/down event, briefly presses Shift and Option, re-sends the event and
releases them. No kernel extension or virtual keyboard driver, no
dependencies, about 300 lines of Rust.

## Install

Needs Rust (`brew install rustup && rustup default stable`).

```sh
git clone https://github.com/Cordazar/fine-volume.git
cd fine-volume
./install.sh
```

This builds the binary, copies it to `~/.local/bin/fine-volume` and loads a
LaunchAgent that starts it at login. Then allow it under **System Settings →
Privacy & Security → Accessibility** (add `~/.local/bin/fine-volume` with
**+** if it isn't listed). It starts working within a couple of seconds;
`/tmp/fine-volume.log` says `fine-volume: running`.

If the steps go back to full size after a rebuild, macOS dropped the
permission: toggle it off and on again.

## Try without installing

```sh
FINE_VOLUME_DEBUG=1 cargo run --release
```

Grant Accessibility to your terminal when asked. With `FINE_VOLUME_DEBUG` set
it logs every media key event it sees.

## Uninstall

```sh
./uninstall.sh
```

## Notes

- It applies to every volume key, built-in or external; macOS doesn't tell an
  event tap which device an event came from.
- A Shift or Option you are physically holding is left alone.

## License

MIT
