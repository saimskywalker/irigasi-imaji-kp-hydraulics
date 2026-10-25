# Contributing

Contributions are welcome. A few things worth knowing before you spend time.

## How changes land

Every change reaches `main` through a pull request that the maintainer reviews
and merges. That includes changes from forks, which is the normal path: you
cannot push to this repository directly, and nothing merges without a review.
Fork the repository, branch from the latest `main`, and open the PR against
`main`. Keep one topic per PR. If a PR sits without a response, a nudge is
fine.

Commit messages follow the short conventional style already in the log:
`fix: ...`, `docs: ...`, `chore: ...`, with the PR number added on merge.

## Before opening a PR

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo doc --no-deps
```

CI runs exactly these four on `ubuntu-latest` with the stable toolchain.
Warnings are denied, and that includes `missing_docs`: a new public item
without a doc comment fails the build.

## AI-assisted contributions

Using AI tools is fine; please just be open about it.

- Say in the PR description which AI tools you used, if any: none, some or most, plus the tool or agent and the model (for example "Claude Code, Opus 5.5" or "Copilot, GPT-5").
- Write the PR description and any issues yourself, in your own words. Short, plain descriptions are welcome; please don't paste generated text.
- Read, run and test every line you submit, and be ready to explain it in review.
- PRs without the disclosure may be closed.

## What a good change looks like

A test built on a worked example with a stated source. Every equation here
follows a KP part, and the test for it should reproduce a published example
from that part or from a well-known hydraulics textbook, with a comment naming
the document, edition and page. A test that only checks the code against
itself does not show the formula is right.

For a bug fix, the test fails without the fix.

## Things to be careful with

**Cite the KP part.** Every new function says in its doc comment which KP part
and section it follows. If you cannot find a source for a value, do not
guess one: leave it as a parameter and say so in the PR.

**Do not reproduce the criteria.** The crate implements equations and
procedures and cites where they come from. It does not copy text out of the KP
documents and does not embed their design tables. A value an engineer reads
off a table or chart is a parameter or a caller-supplied table. A `Default`
with representative values is allowed only when it is documented as a
convenience the engineer must confirm against the current edition.

**Units.** Every quantity is a newtype, and conversions are named methods.
Do not add a function that takes a bare `f64` for a physical quantity, and do
not add a second place where mm/day becomes l/s/ha.

**No panics, no unbounded loops.** Library code returns `Result` with a
`KpError`; `unwrap` and `expect` belong in tests and examples only. Iterative
solvers are bounded and report `DidNotConverge` or `NoBracket` instead of
returning an unconverged answer.

**A failed check does not stop the calculation.** Return the result with the
failure recorded, so the engineer can see by how much a limit is exceeded.

## Scope

Irrigation hydraulics following KP-01 to KP-07. Structural member design
belongs in
[imaji-structural-design](https://github.com/saimskywalker/sni-imaji-structural-design).
The crate has no dependencies beyond the standard library, and a PR that adds
one needs a strong reason.
