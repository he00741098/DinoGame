const { createServer } = require("http");
const { Server } = require("socket.io");
const { callbackify } = require("util");

let ids = 0;
let scores = {};

const httpServer = createServer();
const io = new Server(httpServer, {
  cors: {
    origin: "http://127.0.0.1:8125"
  }
});

io.on("connection", (socket) => {
  ids = ids+1;
  socket.data.id = ids;
  scores[socket.data.id] = 0;


  socket.on("move", () => {
    scores[socket.data.id] = scores[socket.data.id]+1;
  });

  socket.on("getScore", (callback) => {
    callback(scores[socket.data.id]);
  });

  // TODO: send this to client
  socket.on("getAllScores", () => {
    console.log(scores);
  });

  socket.on("disconnect", (reason) => {
    delete scores[socket.data.id];
  });
});

httpServer.listen(6969);