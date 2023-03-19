//const { createServer } = require("http");
//const { Server } = require("socket.io");
var port = port = process.env.PORT || 8125;
//let ids = 0;
let scores = {};

const httpServer = require('http').createServer(function (request, response) {
  console.log('request starting...');

  var filePath = './client/' + request.url;
  console.log(filePath);
  if (filePath == './client//') {
      filePath = './client//index.html';
  }
  filePath = filePath.split("?")[0];
  var extname = path.extname(filePath);
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

  fs.readFile(filePath, function(error, content) {
      if (error) {
          if(error.code == 'ENOENT'){
              fs.readFile('./404.html', function(error, content) {
                  response.writeHead(404, { 'Content-Type': contentType });
                  response.end(content, 'utf-8');
              });
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
    if(typeof scores[username] != "undefined") {
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

httpServer.listen(port);
console.log('Server running at http://127.0.0.1:'+port);