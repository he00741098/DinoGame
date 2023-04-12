const host = "he00741098-supreme-lamp-pq7gq4jpprj25gq-8125.preview.app.github.dev/";

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