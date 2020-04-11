# A simple `!Unpin` I/O backend for async-std

This is a wrapper around `async-std`'s Cursor, but this one is `!Unpin`.
I wanted it for tests for async I/O code that was supposed to be able to support both `Unpin` and `!Unpin` backends.

See the crate-level documentation for usage info and examples.
