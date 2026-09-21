# me - facilitate being myself

## Installation

### macOS

1. Download the latest `.dmg` from the [Releases page](https://github.com/FredPerr/me/releases/latest).
2. Open the `.dmg` and drag **me** into your `Applications` folder.

Because the app is not signed with an Apple Developer certificate, macOS will block
it the first time ("me" is damaged / cannot be opened / from an unidentified developer).
Clear the quarantine attribute to allow it to run:

```sh
xattr -dr com.apple.quarantine /Applications/me.app
```

Then open the app normally from `Applications`.

## Features

### Shortcuts
To focus/open the `me`, use `Cmd` + `Shift` + `m`

## License

Copyright © 2026 Frédéric Perron.

This project is source-available under the [PolyForm Noncommercial License 1.0.0](./LICENSE).
You are free to use, study, and modify it for any noncommercial purpose (personal use,
hobby projects, research, and similar). Commercial use is not permitted without a separate
license from the copyright holder.
