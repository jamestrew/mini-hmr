# mini-hmr

Hot Module Replacement (HMR) seems pretty magical.
I'm gonna build my own very simple one just to demystify things for myself.

## The Idea

### dev server

- serves the html/js/css files
- watches these files for changes
- also has a websocket connection to the client

### client

has some sort of client code specifically for the hmr that
- listens on the websocket connection
- reacts to update messages on the ws, refetches the updated file and does the module updating
- can hook into things like react-refresh for updating react components without losing state

### update protocol

some protocol from communicating updates from the dev server to the client
eg vite:
```
{"type": "connected"}
{"type": "ping"}  // just some heartbeat?
{"type": "update", "updates": [{"type": ..., "path": ..., "timestamp": ...]}
```

## The Plan

- [ ] throw together some webserver
- [ ] set up some sort of file watching (will need debounce) and just log it out to the server logs
- [ ] set up ws connection and basic client code and log file update to console
- [ ] some sort of premitive reloading on the client side, fetch the new file and run it?
- [ ] need to study client hmr code more for more advanced stuff


