const host = "he00741098-curly-space-lamp-rv7rvq4pwv5hx9wq-8125.preview.app.github.dev/";

let websocket = new WebSocket('wss://'+host);
websocket.onmessage = function (event) {
    console.log(event.data);
    }
websocket.onopen = function (event) {
    console.log('Connected to server');
    let message = {name: "checkPlayerUsername", args: ["name"]};
    let json = JSON.stringify(message);
    websocket.send(json);
}