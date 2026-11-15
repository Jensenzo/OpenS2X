# Security

## Reporting a problem

Please report a security problem privately, not in an issue: on the
repository's **Security** tab, **Report a vulnerability** (GitHub's private
vulnerability reporting). Only the maintainer sees the report. Say what is
affected, how to reproduce it, and which version you used. You will get an
answer, and a fix goes into the next release, with credit if you want it.

## What is in scope

OpenS2X has no network code, no telemetry and no updater. What it reads from
outside is files: the game's own files, the player's profile and settings,
and saves, including the original's `.sav` files and saves someone else
gives you. A file that makes the program crash, hang, use memory without
bound or write outside its folders is worth reporting.

Only the latest release is supported.
