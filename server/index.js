//const { createServer } = require("http");
//const { Server } = require("socket.io");
var fs = require('fs');
var path = require('path');
var port = process.env.PORT || 8125;

let scores = [];
let rooms = {};
let gameStarted = false;
let gameSeed = 0;
const httpServer = require('http').createServer(function (request, response) {
  console.log('request starting...');

  let filePath = request.url.split("?")[0];

  let safeSuffix = path.normalize(filePath).replace(/^(\.\.(\/|\\|$))+/, '');
  let safePath = path.join("./client/", safeSuffix);

  if(safePath === "client/") {
    safePath = "client/index.html";
  }

  console.log(safePath);

  var extname = path.extname(safePath);
  var contentType = 'text/html';
  switch (extname) {
      case '.js':
          contentType = 'text/javascript';
          break;
      case '.css':
          contentType = 'text/css';
          break;
      case '.json':
          contentType = 'application/json';
          break;
      case '.png':
          contentType = 'image/png';
          break;      
      case '.jpg':
          contentType = 'image/jpg';
          break;
      case '.wav':
          contentType = 'audio/wav';
          break;
  }

  fs.readFile(safePath, function(error, content) {
      if (error) {
          if(error.code == 'ENOENT'){
            response.writeHead(404);
            response.end("Not found");
          }
          else {
              response.writeHead(500);
              response.end('Sorry, check with the site admin for error: '+error.code+' ..\n');
              response.end(); 
          }
      }
      else {
          response.writeHead(200, { 'Content-Type': contentType });
          response.end(content, 'utf-8');
      }
  });

});
//createServer();
const io = require('socket.io')(httpServer);
// new Server(httpServer, {
//   cors: {
//     origin: "http://127.0.0.1:8125"
//   }
// });

io.on("connection", (socket) => {
  //ids = ids+1;
  //socket.data.id = ids;
  //scores[socket.data.id] = 0;


  socket.on("registerPlayer", (username, roomid, callback) => {
    if(typeof callback !== "function") {
      return;
    }
    if(socket.data.registered) {
      callback("EALREADYREG");
      return;
    }
    if(typeof username !== "string") {
      callback("EBADREQ");
      return;
    }
    if(typeof roomid !== "string") {
      callback("EBADREQ");
      return;
    }
    if(typeof scores[username] != "undefined") {
      callback("ETAKEN");
      return;
    }

    if(roomid != "global") {
      if(typeof rooms[roomid] == "undefined") {
        rooms[roomid] = {started:false, host:username};
      }
      if(rooms[roomid].started) {
        callback("EALREADYSTARTED");
        return;
      }
    }

    socket.join(roomid);
    socket.data.id = username;
    socket.data.room = roomid;
    socket.data.registered = true;
    scores[socket.data.id] = {score:0, pos:0, room:roomid, username:username};
    callback("SUCCESS");
  });
//Object.values(scores).filter((h) => h.roomid == "hello"); 
  socket.on("startRoom", (callback) => {
    if(typeof callback !== "function") {
      return;
    }
    if(!socket.data.registered) {
      callback("ENOTREG");
      return;
    }
    if(socket.data.room == "global") {
      callback("EBADREQ")
      return;
    }
    if(typeof rooms[socket.data.room] == "undefined") {
      callback("EBADREQ");
      return;
    }
    if(rooms[socket.data.room].host != socket.data.id) {
      callback("ENOTHOST");
      return;
    }
    rooms[socket.data.room].started = true;
    socket.to(socket.data.room).emit("startGame");
  });

  socket.on("checkPlayerStatus", (username, roomid, callback) => {
    if(typeof callback !== "function") {
      return;
    }
    if(typeof username !== "string") {
      callback("EBADREQ");
      return;
    }
    if(typeof roomid !== "string") {
      callback("EBADREQ");
      return;
    }
    if(typeof rooms[roomid] !== "undefined") {
      if(rooms[roomid].started) {
        callback("EALREADYSTARTED");
        return;
      }
    }

    if(typeof scores[username] != "undefined") {
      callback("TAKEN");
    } else {
      callback("NOT TAKEN");
    }
  });

  socket.on("move", () => {
    if(!socket.data.registered) {
      return;
    }
    scores[socket.data.id]["score"] = scores[socket.data.id]["score"]+1;
  });

  socket.on("getStarted", (callback) => {
    if(typeof callback !== "function") {
      return;
    }
    if(!socket.data.registered) {
      callback("ENOTREG");
      return;
    }
    if(socket.data.room == "global") {
      callback("EBADREQ");
      return;
    }
    if(typeof rooms[socket.data.room] !== "undefined") {
      if(rooms[socket.data.room].started) {
        callback("started");
        return;
      }
    }
  });

  socket.on("checkHost", (callback) => {
    if(typeof callback !== "function") {
      return;
    }
    if(!socket.data.registered) {
      callback("ENOTREG");
      return;
    }
    if(socket.data.room == "global") {
      callback("EBADREQ");
      return;
    }
    if(typeof rooms[socket.data.room] !== "undefined") {
      if(rooms[socket.data.room].host == socket.data.id) {
        callback("HOST");
        return;
      }
      callback("NOT HOST");
      return;
    }
  });

  socket.on("getScore", (callback) => {
    //Object.values(scores).filter((h) => h.roomid == socket.data.room);
    if(typeof callback !== "function") {
      return;
    }
    if(!socket.data.registered) {
      callback("ENOTREG");
      return;
    }
    callback(scores[socket.data.id]["score"]);

  });

  socket.on("getAllScores", (callback) => {
    if(typeof callback !== "function") {  
      return;
    }
    let filter = Object.values(scores).filter((h) => h.room == socket.data.room); // NEVER CALLED BECASUE SCORE SOCKET IS SEPERATE
    let score_compat = [];
    for (let i of filter) {
      score_compat.push({username: i.username, score: i.score});
    }
    callback(score_compat);
  });

  socket.on("disconnect", () => {
    delete scores[socket.data.id];
  });

  socket.on("endPlayer", () => {
    delete scores[socket.data.id];
    socket.data.registered = false;
  });

  socket.on("sendPos", (pos, callback) => {
    if(typeof callback !== "function") {
      return;
    }
    if(typeof pos !== "number") {
      callback("EBADREQ");
      return;
    }
    if(!socket.data.registered) {
      callback("ENOTREG");
      return;
    }
    scores[socket.data.id]["pos"] = pos;
  });

  socket.on("getAllPos", (callback) => {
    if(typeof callback !== "function") {
        return;
    }
    let filter = Object.values(scores).filter((h) => h.room == socket.data.room);
    let pos_compat = [];
    for (let i of filter) {
      pos_compat.push({username: i.username, pos: i.pos});
    }
    callback(JSON.stringify(pos_compat));


  });


  socket.on("getObstacles", (callback)=>{
    if(typeof callback !== "function"){
      return;
    }
    if(!gameStarted){
      //gameSeed = generateTerrain(100000);
        let length = 100000;
        let obstacle_distance = 1000;
        let obstacles = [];
        for(i=1106; i<length; i+=obstacle_distance){
          obstacle_distance = Math.floor(Math.random()*1000)+300;
            //let sprite = new PIXI.AnimatedSprite(spritesheet.animations.cactus);
//sprite.x = app.view.width;
            //sprite.y = defY-sprite.height;
            //sprite.height = sprite.height*0.25;
            //sprite.width = sprite.width*0.25;
            //obstacles.push(new obstacle(i, 0, 5, 2, sprite));


            let alphabet = ["a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l", "m", "n", "o", "p", "q", "r", "s", "t", "u", "v", "w", "x", "y", "z"];
            obstacles.push({type:alphabet[Math.floor(Math.random()*alphabet.length)], x:i, y:0})
            //console.log("added new obstacle at "+i+","+0);
        }      
      gameSeed = obstacles;
      gameStarted = true;
    }
    //console.log("sending game seed: "+JSON.stringify(gameSeed));
    callback(JSON.stringify(gameSeed));
  });





});

httpServer.listen(port);
console.log('Server running at http://127.0.0.1:'+port);