//const { createServer } = require("http");
//const { Server } = require("socket.io");
var fs = require('fs');
var path = require('path');
var port = process.env.PORT || 8125;

let scores = [];
let gameStarted = false;
let gameSeed = 0;
const httpServer = require('http').createServer(function (request, response) {
  console.log('request starting...');

  let filePath = request.url.split("?")[0];

  let safeSuffix = path.normalize(filePath).replace(/^(\.\.(\/|\\|$))+/, '');
  let safePath = path.join("./client-rust/", safeSuffix);

  if(safePath === "client-rust/") {
    safePath = "client-rust/index.html";
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

httpServer.listen(port);
console.log('Server running at http://127.0.0.1:'+port);