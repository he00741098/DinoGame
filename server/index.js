const { createServer } = require("http");
const { Server } = require("socket.io");

//let ids = 0;
let scores = {};

const httpServer = createServer();
const io = new Server(httpServer, {
  cors: {
    origin: "http://127.0.0.1:8125"
  }
});

io.on("connection", (socket) => {
  //ids = ids+1;
  //socket.data.id = ids;
  //scores[socket.data.id] = 0;


  socket.on("registerPlayer", (username, callback) => {
    if(typeof callback !== "function") {
      return;
    }
    if(socket.data.registered) {
      callback("EALREADYREG");
      return;
    }
    if(typeof username !== "string") {
      callback("EBADREQ")
      return;
    }
    if(scores[username]) {
      callback("ETAKEN");
      return;
    }
    socket.data.id = username;
    socket.data.registered = true;
    scores[socket.data.id] = 0;
    callback("SUCCESS");
  });

  socket.on("checkPlayerUsername", (username, callback) => {
    if(typeof callback !== "function") {
      return;
    }
    if(typeof username !== "string") {
      callback("EBADREQ")
      return;
    }
    if(typeof scores[username] != "undefined") {
      callback("TAKEN");
    } else {
      callback("NOT TAKEN")
    }
  });

  socket.on("move", () => {
    if(!socket.data.registered) {
      return;
    }
    scores[socket.data.id] = scores[socket.data.id]+1;
  });

  socket.on("getScore", (callback) => {
    if(typeof callback !== "function") {
      return;
    }
    if(!socket.data.registered) {
      callback("ENOTREG");
      return;
    }
    callback(scores[socket.data.id]);
  });

  socket.on("getAllScores", (callback) => {
    if(typeof callback !== "function") {  
      return;
    }
    callback(scores);
  });

  socket.on("disconnect", (reason) => {
    delete scores[socket.data.id];
  });
});

httpServer.listen(6969);