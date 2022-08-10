# Rail distances map

This project is a hobby project to create a visualisation of travel times between train stations in the UK. It consists of a Rust backend which loads and parses RDG timetable data and implements Dijkstras to solve minimum journey times, and a Vue.js frontend with a UK map which lets the user pick stations and shows travel time links between every station.

This was largely done as an exercise in Vue, Rust, and rail timetable data. As such the codebase is largely abandoned - I don't recommend trying to make this work but if you do, the approximate process is:

1. Create an account with the Rail Delivery Group and use the `Starter/download_timetables.py` script to download the latest timetable data.
2. Run the backend with `cd raildata/railserver && cargo run`
3. Run the frontend with `cd web/webclient && npm install && npm run serve`
