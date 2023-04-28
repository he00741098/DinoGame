# Procedure

Make the thing work

- Websockets with tokio-tungstenite
- Tokio async


How to make it work

- Main function starts listening for connections

- Send connection to processing thread / function
- Connection opens -> return the socket
- Listen for message
- JSON -> ClientCommand (enum or struct??)


- Join room command -> tries to join a room
- Join quickplay command -> Joins quickplay
    - Starts pinging server for countdown.
    - Server responds with count down??
    - Everyone starts together.
    - Connect to the "room"


- New room thread???
- room thread manages the game, starts the processing async thread
- Processing async thread keeps track of player positions with a 2d array
- Player async thread keeps track of array indexes in "Cache"
- Proccessing async thread manages player speeds.


Task Division
-
- Main thread -> Listen for connections and send them to a new thread/task
- Main thread -> start room thread -> proccess rooms
