# Redactify desktop app

This directory is the Redactify desktop app, built with
[Tauri](https://tauri.app/):

- `src-tauri/` is the Rust side: Tauri commands, session state, and native
  menus. It depends on `redactify-core` for all detection and redaction.
- `src/` is the React and TypeScript review interface.

## Commands

Run these from this directory:

```console
$ npm ci                      # install dependencies exactly as locked
$ npm run tauri dev           # run the app with hot reload
$ npm run build               # type-check and build the frontend, as CI does
```

`npm run dev` starts only the frontend, without the Rust backend, so the app
cannot open files or scan anything. Use `npm run tauri dev` for development.

## Documentation

- [Using the desktop app](../docs/user-guide/desktop-app.md)
- [Architecture](../docs/development/architecture.md)
- [Contributing](../CONTRIBUTING.md)
