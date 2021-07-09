# tiny-serve
A small, fast, fully asynchronous, and concurrent HTTP/1.1 server for static content stored on the filesystem, written in Rust using `async_std` and `futures`. 

It uses a custom hand-written parser based on the relevant RFCs that sweeps over the message with a single byte of lookahead. The parser supports a reasonable subset of HTTP/1.1, but lacks some unnecessary grammar and such that are only used for optional, unimplemented HTTP/1.1 verbs.

It accepts one command line argument, the port number to bind to:

```
tiny-serve 8080
```

which defaults to port 8000 if not specified or invalid.