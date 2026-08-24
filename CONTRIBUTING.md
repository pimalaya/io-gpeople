# Contributing guide

Thank you for investing your time in contributing to I/O Google People.

Whether you are a human or an AI agent, read these in order before touching the code:

1. the [Pimalaya README](https://github.com/pimalaya) for what the project is and how its repositories stack;
2. the [Pimalaya CONTRIBUTING](https://github.com/pimalaya/.github/blob/master/CONTRIBUTING.md) guide, which chains to the shared architecture and guidelines;
3. the inline header documentation, starting with src/lib.rs: it is the architecture document of this crate;
4. the docs/ folder for the development history and living plans.

Everything below documents only what differs from the Pimalaya standards.

## Feature matrix

io-gpeople follows the standard layered split (I/O-free coroutines, then the std client behind the `client` feature), plus a vendored switch compiling the TLS dependencies from source:

```sh
cargo build --no-default-features                        # coroutines only, no std leak
cargo build --no-default-features --features client      # light client, no TLS deps
cargo build                                              # full client (rustls-ring by default)
cargo build --no-default-features --features rustls-aws  # full client, aws-lc-rs crypto
cargo build --no-default-features --features native-tls  # full client, platform TLS
cargo build --features vendored                          # vendored TLS dependencies
```

## Tests

The default suite is fully offline: every coroutine is driven against an in-memory stream replaying a canned HTTP response, so no network access nor OAuth token is required.

```sh
cargo test
```

tests/people.rs is an ignored end-to-end test walking the whole CRUD surface against the live People API. It needs a TLS feature and a `PEOPLE_ACCESS_TOKEN` environment variable, and it deletes everything it creates:

```sh
PEOPLE_ACCESS_TOKEN=<token> cargo test --test people -- --include-ignored
```
