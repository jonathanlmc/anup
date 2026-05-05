# anup

This branch contains a rewrite of the original application, using more current Rust idioms and crates.

The primary goal of this rewrite is to to develop a TUI interface with similar overall functionality to what is currently present on `master`, but with various quality of life improvements.

The biggest improvements are centered around series detection. More specifically:

* Async detection to avoid blocking navigation on the interface while a series resolves.
* Detect more episode filename layouts out-of-the-box.
* Use top-level directories as a series root, instead of the directory closest to the episode files.
  As an example, this means that a directory named `Series Name` containing `S1` and `S2` subfolders would now have a single series entry for the `Series Name` directory, instead of having one for each subfolder.
* Automatically handle "merged-seasons" transparently.
* Remove the need to create a nickname / alias for each added series.
* Automatically handle adding and removing a series from the program (with an option to disable both).

This list is off the top of my head and may be incomplete or altered later, but those items would resolve some of the biggest pain points I feel the version on `master` has, based off personal use.
