const host = "he00741098-effective-acorn-pq7gq4jpp9wc7xrp-8125.preview.app.github.dev/";

let websocket = new WebSocket('wss://'+host);
websocket.onmessage = function (event) {
    console.log(event.data);
    }
websocket.onopen = function (event) {
    console.log('Connected to server');
    let len = Math.floor(Math.random()*20);
    let name = "";
    for(let g = 0; g<len; g++){
        name+=Math.floor(Math.random()*10);
    }

    let message = {"RegPlayer" : name};
    let json = JSON.stringify(message);
    websocket.send(json);
    message = {"JoinRoom" : "Hi"};
    json = JSON.stringify(message);
    websocket.send(json);
    message = "LeaveRoom";
    json = JSON.stringify(message);
    websocket.send(json);
    message = "QuickPlay";
    json = JSON.stringify(message);
    websocket.send(json);
    message = "GetData";
    json = JSON.stringify(message);
    websocket.send(json);
    message = "GetRoomList";
    json = JSON.stringify(message);
    websocket.send(json);
    message = "Ready";
    json = JSON.stringify(message);
    websocket.send(json);
}