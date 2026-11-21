---
name: pull request
about: changes for review
title: ''
labels: ''
assignees: ''
---

what does this change, in one sentence?

why is it needed? link the issue if there is one.

how did you verify it? paste the commands you actually ran, not the ones you
meant to run.

```sh

```

if this touches the worker, the download path, or the layout, explain the
trade-off you picked. those are the three places where the obvious fix is
usually the wrong one.

checklist (delete the ones that do not apply):

- [ ] `cargo test` passes
- [ ] `cargo clippy --all-targets -- -D warnings` is clean
- [ ] `cargo fmt --all -- --check` is clean
- [ ] docs updated if behaviour changed
