# AstralEra

a server program for a ~~critically acclaimed~~ MMORPG that shut down in the early 2010s.

## Subprojects

- **common:** a library that provides common shared components most or all other subprojects utilize.
- **database:** provides an abstract layer for accessing AstralEra's database.
- **launcher:** launcher program. takes user login data, ensures the game files are up-to-date, and starts the game.
- **server_lobby:** handles data associated with user accounts such as characters, and other functionality related to the game's main menu and lobby.
- **server_login:** handles server components relevant to the launcher- primarily user registration & login.

some of these projects have readmes further detailing their functionality; check their individual directories.

## Questions

### Okay, but *why*?

principally? it's for preservation- as awful as the original game was, it is not playable in any form outside a handful unfinished server recreation projects.

but also, it's just fun. wouldn't *you* like to work on a project that makes people say "okay, but *why*?" when you explain it to them?

### What's it made with?

AstralEra is built with [Rust](https://www.rust-lang.org/).

### How complete is it?

as of writing this readme, it can not even launch fully into the game. it is a *very* early-stages project.

## License & Credit

AstralEra is licensed under the terms of the [GNU GPL v3](LICENSE).

AstralEra builds on the research and effort of [The Seventh Umbral](http://seventhumbral.org) and [Project Meteor](http://ffxivclassic.fragmenterworks.com/wiki/index.php/Main_Page). their effort into deciphering game data and server functionality is an invaluable resource in this project.