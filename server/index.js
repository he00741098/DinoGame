const { Server } = require("socket.io");

const io = new Server(6969, { /* options */ });

io.on("connection", (socket) => {
  // ...
});