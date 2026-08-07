# Preferences

How the owner wants this project worked on. Stated by them, not inferred.

---

## Verify for real, every time

> "เขียนโค้ดและทดสอบใช้งานจริง ว่าปกติไหมตลอดนะ"

Green tests are not evidence the feature works. Run the binary, do the thing a user would do,
and report the actual output. If a step was skipped or a test failed, say so plainly rather
than reporting success.

This is why `PLAN.md` carries an explicit smoke test per phase, and why `tests/cli.rs`
invokes the real binary instead of unit-testing the dispatch function.

## Keep the docs moving with the code

> "พร้อมอัพเดทเอกสารสม่ำเสมอ"

`PLAN.md`, `README.md`, `CHANGELOG.md`, and the relevant `.knowledge/` or `.learning/` file
update **in the same commit** as the behavior they describe — not in a follow-up pass.

## Study before building

The owner asked for the full survey of the reference agents to be read and written up before
any real code was written, and pushed back when the first pass was too shallow ("อ่านทั้งหมด
ก่อนแล้วค่อยทำ"). Read the primary source; do not skim layouts and infer.

## Names must not need explaining

Two names were rejected for being hard to remember (`hawser`, `tug` — the first needed a
dictionary). Short, and obvious without a gloss.

## Language

The owner writes in Thai and expects replies in Thai. Code, comments, commit messages, and
documentation in this repo are in English.
