const { createServer } = require("http");
const { Server } = require("socket.io");

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


  socket.on("scoreSend", (...score) => {
    scores[socket.data.id] = score[0];
  });

  // TODO: send this to client
  socket.on("getAllScores", () => {
    console.log(scores);
  });

  // TODO: maybe remove score from object on disconnect
});

httpServer.listen(6969);