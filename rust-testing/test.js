let WebSocket = require("ws");
let socketList = [];
console.log("Starting");

for (let i = 0; i < 2; i++) {
    let socket = new WebSocket("wss://rustdinogame.herokuapp.com/");
    
    socket.onopen = function() {
        socketList.push(socket);
        y(socket, i);
        setInterval(d, 800);
        setInterval(b, 500);
    };
    socket.onmessage = function(data) {
        console.log(data.data);
    };

}
function y(socket, i){
    socket.send(JSON.stringify({"RegPlayer": "test"+i}));
}

function d(){
    c();
    x();
}
function win(){
    socketList[0].send(JSON.stringify({"PostPos":100000}));
}

function x(){
    for (let i = 0; i < socketList.length; i++) {
        socketList[i].send(JSON.stringify("QuickPlay"));
        socketList[i].send(JSON.stringify("Ready"));
    }
    if(Math.random() > 0.8){
        win();
    }

    //socket.send(JSON.stringify("QuickPlay"));
    //socket.send(JSON.stringify("Ready"));
}

function b(){
    for (let i = 0; i < socketList.length; i++) {
        socketList[i].send(JSON.stringify("GetData"));
    }
}
function c(){
    for (let i = 0; i < socketList.length; i++) {
        socketList[i].send(JSON.stringify("LeaveRoom"));
    }
}