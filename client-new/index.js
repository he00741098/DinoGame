const host = "he00741098-effective-eureka-5pv4pjgx5xv3xp5-8125.preview.app.github.dev/";

let websocket = new WebSocket('wss://'+host);
websocket.onmessage = function (event) {
    console.log(event.data);
    }
websocket.onopen = function (event) {
    console.log('Connected to server');
    let message = {"RegPlayer" : "Hi"};
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